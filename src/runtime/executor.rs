use std::sync::Arc;
use anyhow::anyhow;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::warn;

use crate::agent::{AgentMessage, AgentRegistry};
use crate::error::{AgentFlowError, Result};
use crate::flow::Flow;
use crate::tools::{ToolRegistry, orchestrator::ToolOrchestrator};
use crate::state::FlowContext;

use super::types::{FlowEvent, FlowExecution, TaskResult, TaskFinished};
use super::state::SharedState;
use super::processor::process_event;

/// Flow 执行器
#[derive(Clone)]
pub struct FlowExecutor {
    flow: Arc<Flow>,
    agents: Arc<AgentRegistry>,
    tools: Arc<ToolRegistry>,
    max_iterations: u32,
    max_concurrency: usize,
    tool_orchestrator: Option<Arc<ToolOrchestrator>>,
}

impl FlowExecutor {
    pub fn new(flow: Flow, agents: AgentRegistry, tools: ToolRegistry) -> Self {
        Self {
            flow: Arc::new(flow),
            agents: Arc::new(agents),
            tools: Arc::new(tools),
            max_iterations: 256,
            max_concurrency: 8,
            tool_orchestrator: None,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: u32) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub fn with_max_concurrency(mut self, limit: usize) -> Self {
        self.max_concurrency = limit.max(1);
        self
    }

    pub fn with_tool_orchestrator(mut self, orchestrator: Arc<ToolOrchestrator>) -> Self {
        self.tool_orchestrator = Some(orchestrator);
        self
    }

    pub async fn start(
        &self,
        ctx: Arc<FlowContext>,
        initial: AgentMessage,
    ) -> Result<FlowExecution> {
        // 只在启用调试模式时输出详细信息
        let debug_mode = std::env::var("AGENTFLOW_DEBUG").is_ok();
        if debug_mode {
            use std::io::{self, Write};
            eprintln!("🚀 FlowExecutor::start() 开始");
            eprintln!("   工作流名称: {}", self.flow.name);
            eprintln!("   起始节点: {}", self.flow.start);
            io::stderr().flush().ok();
        }
        
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        let start_node = self.flow.start.clone();
        tx.send(FlowEvent {
            node: start_node.clone(),
            message: initial,
            iterations: 0,
            trace_id: crate::agent::message::uuid(),
            source: "__start__".to_string(),
        })
        .map_err(|_| AgentFlowError::Other(anyhow!("failed to enqueue initial event")))?;

        let mut join_set: JoinSet<Result<TaskResult>> = JoinSet::new();
        let mut inflight = 0usize;
        let mut finished: Option<FlowExecution> = None;
        let collected_errors: Vec<crate::error::FrameworkError> = Vec::new();
        let shared = Arc::new(SharedState::default());

        while finished.is_none() {
            tokio::select! {
                Some(result) = join_set.join_next(), if inflight > 0 => {
                    inflight -= 1;
                    match result {
                        Ok(Ok(TaskResult::Continue)) => {},
                        Ok(Ok(TaskResult::Finished(data))) => {
                            finished = Some(FlowExecution {
                                flow_name: self.flow.name.clone(),
                                last_node: data.node,
                                last_message: data.message,
                                errors: collected_errors.clone(),
                            });
                        }
                        Ok(Err(error)) => return Err(error),
                        Err(join_error) => return Err(AgentFlowError::Other(join_error.into())),
                    }
                }
                Some(event) = rx.recv(), if finished.is_none() => {
                    if debug_mode {
                        use std::io::{self, Write};
                        eprintln!("📥 接收到新事件: {} (来源: {})", event.node, event.source);
                        io::stderr().flush().ok();
                    }
                    if inflight >= self.max_concurrency {
                        if let Some(result) = join_set.join_next().await {
                            inflight -= 1;
                            match result {
                                Ok(Ok(TaskResult::Continue)) => {}
                                Ok(Ok(TaskResult::Finished(data))) => {
                                    finished = Some(FlowExecution {
                                        flow_name: self.flow.name.clone(),
                                        last_node: data.node,
                                        last_message: data.message,
                                        errors: collected_errors.clone(),
                                    });
                                }
                                Ok(Err(error)) => return Err(error),
                                Err(join_error) => return Err(AgentFlowError::Other(join_error.into())),
                            }
                        }
                    }

                    if finished.is_none() {
                        let flow = Arc::clone(&self.flow);
                        let agents = Arc::clone(&self.agents);
                        let tools = Arc::clone(&self.tools);
                        let ctx = Arc::clone(&ctx);
                        let sender = tx.clone();
                        let max_iterations = self.max_iterations;
                        let shared = Arc::clone(&shared);
                        let tool_orchestrator = self.tool_orchestrator.clone();

                        if debug_mode {
                            use std::io::{self, Write};
                            eprintln!("   🔄 启动任务处理事件 (inflight: {})", inflight);
                            io::stderr().flush().ok();
                        }
                        join_set.spawn(async move {
                            process_event(
                                event,
                                flow,
                                agents,
                                tools,
                                ctx,
                                sender,
                                max_iterations,
                                tool_orchestrator,
                                shared,
                            )
                            .await
                        });
                        inflight += 1;
                    }
                }
                else => {
                    if inflight == 0 {
                        break;
                    }
                }
            }
        }

        drop(rx);
        drop(tx);

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(TaskResult::Continue)) => {}
                Ok(Ok(TaskResult::Finished(data))) => {
                    if finished.is_none() {
                        finished = Some(FlowExecution {
                            flow_name: self.flow.name.clone(),
                            last_node: data.node,
                            last_message: data.message,
                            errors: collected_errors.clone(),
                        });
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(join_error) => return Err(AgentFlowError::Other(join_error.into())),
            }
        }

        finished.ok_or_else(|| AgentFlowError::Other(anyhow!("flow finished without result")))
    }
}

