use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, warn};

use crate::agent::{
    AgentAction, AgentContext, AgentMessage, AgentRegistry, AgentRuntime, MessageRole,
};
use crate::error::{AgentFlowError, Result};
use crate::flow::{Flow, FlowNodeKind};
use crate::state::FlowContext;
use crate::tools::ToolRegistry;

#[derive(Clone)]
pub struct FlowExecutor {
    flow: Arc<Flow>,
    agents: Arc<AgentRegistry>,
    tools: Arc<ToolRegistry>,
    max_iterations: u32,
    max_concurrency: usize,
}

impl FlowExecutor {
    pub fn new(flow: Flow, agents: AgentRegistry, tools: ToolRegistry) -> Self {
        Self {
            flow: Arc::new(flow),
            agents: Arc::new(agents),
            tools: Arc::new(tools),
            max_iterations: 256,
            max_concurrency: 8,
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

    pub async fn start(
        &self,
        ctx: Arc<FlowContext>,
        initial: AgentMessage,
    ) -> Result<FlowExecution> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(FlowEvent {
            node: self.flow.start.clone(),
            message: initial,
            iterations: 0,
        })
        .map_err(|_| AgentFlowError::Other(anyhow!("failed to enqueue initial event")))?;

        let mut join_set: JoinSet<Result<TaskResult>> = JoinSet::new();
        let mut inflight = 0usize;
        let mut finished: Option<FlowExecution> = None;

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
                            });
                        }
                        Ok(Err(error)) => return Err(error),
                        Err(join_error) => return Err(AgentFlowError::Other(join_error.into())),
                    }
                }
                Some(event) = rx.recv(), if finished.is_none() => {
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

                        join_set.spawn(async move {
                            process_event(
                                event,
                                flow,
                                agents,
                                tools,
                                ctx,
                                sender,
                                max_iterations,
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

#[derive(Clone)]
struct FlowEvent {
    node: String,
    message: AgentMessage,
    iterations: u32,
}

enum TaskResult {
    Continue,
    Finished(TaskFinished),
}

struct TaskFinished {
    node: String,
    message: Option<AgentMessage>,
}

async fn process_event(
    event: FlowEvent,
    flow: Arc<Flow>,
    agents: Arc<AgentRegistry>,
    tools: Arc<ToolRegistry>,
    ctx: Arc<FlowContext>,
    sender: mpsc::UnboundedSender<FlowEvent>,
    max_iterations: u32,
) -> Result<TaskResult> {
    if event.iterations >= max_iterations {
        return Err(AgentFlowError::MaxIterationsExceeded(max_iterations));
    }

    ctx.push_message(event.message.clone());

    let node = flow
        .node(&event.node)
        .ok_or_else(|| AgentFlowError::UnknownNode(event.node.clone()))?;

    match &node.kind {
        FlowNodeKind::Terminal => {
            debug!("Reached terminal node `{}`", node.name);
            return Ok(TaskResult::Finished(TaskFinished {
                node: node.name.clone(),
                message: Some(event.message.clone()),
            }));
        }
        FlowNodeKind::Agent(agent_name) => {
            let agent = agents
                .get(agent_name)
                .ok_or_else(|| AgentFlowError::AgentNotRegistered(agent_name.clone()))?;
            let runtime_handle = ExecutorRuntime {
                ctx: Arc::clone(&ctx),
                tools: Arc::clone(&tools),
            };
            let agent_ctx = AgentContext {
                flow_ctx: &ctx,
                runtime: &runtime_handle,
            };

            let action = agent.on_message(event.message.clone(), &agent_ctx).await?;
            handle_action(action, &event, flow, &ctx, &tools, sender).await
        }
    }
}

async fn handle_action(
    action: AgentAction,
    event: &FlowEvent,
    flow: Arc<Flow>,
    ctx: &Arc<FlowContext>,
    tools: &Arc<ToolRegistry>,
    sender: mpsc::UnboundedSender<FlowEvent>,
) -> Result<TaskResult> {
    match action {
        AgentAction::Next { target, message } => {
            enqueue_event(sender, target, message, event.iterations + 1)?;
            Ok(TaskResult::Continue)
        }
        AgentAction::Branch { branches } => {
            let mut dispatched = false;
            for (target, message) in branches {
                if flow.node(&target).is_some() {
                    enqueue_event(sender.clone(), target, message, event.iterations + 1)?;
                    dispatched = true;
                }
            }
            if dispatched {
                Ok(TaskResult::Continue)
            } else {
                warn!("No valid branch found, stopping flow");
                Ok(TaskResult::Finished(TaskFinished {
                    node: event.node.clone(),
                    message: None,
                }))
            }
        }
        AgentAction::CallTool {
            tool,
            invocation,
            on_complete,
        } => {
            let runtime_handle = ExecutorRuntime {
                ctx: Arc::clone(ctx),
                tools: Arc::clone(tools),
            };
            let tool_message = runtime_handle.call_tool(&tool, invocation).await?;
            ctx.push_message(tool_message.clone());

            if let Some(target) = on_complete {
                enqueue_event(sender, target, tool_message, event.iterations + 1)?;
                Ok(TaskResult::Continue)
            } else {
                Ok(TaskResult::Finished(TaskFinished {
                    node: event.node.clone(),
                    message: Some(tool_message),
                }))
            }
        }
        AgentAction::Finish { message } => {
            if let Some(msg) = &message {
                ctx.push_message(msg.clone());
            }
            Ok(TaskResult::Finished(TaskFinished {
                node: event.node.clone(),
                message,
            }))
        }
        AgentAction::Continue { message } => {
            let transitions = next_from_flow(&event.node, &flow, ctx).await?;
            if transitions.is_empty() {
                return Ok(TaskResult::Finished(TaskFinished {
                    node: event.node.clone(),
                    message,
                }));
            }

            for (target, default_message) in transitions {
                let to_send = message
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| default_message.clone());
                enqueue_event(sender.clone(), target, to_send, event.iterations + 1)?;
            }
            Ok(TaskResult::Continue)
        }
    }
}

fn enqueue_event(
    sender: mpsc::UnboundedSender<FlowEvent>,
    target: String,
    message: AgentMessage,
    iterations: u32,
) -> Result<()> {
    if sender
        .send(FlowEvent {
            node: target,
            message,
            iterations,
        })
        .is_err()
    {
        warn!("scheduler channel closed before event could be enqueued");
    }
    Ok(())
}

async fn next_from_flow(
    node_name: &str,
    flow: &Arc<Flow>,
    ctx: &Arc<FlowContext>,
) -> Result<Vec<(String, AgentMessage)>> {
    let mut results = Vec::new();
    for transition in flow.transitions(node_name) {
        if let Some(condition) = &transition.condition {
            if !(condition)(ctx).await {
                continue;
            }
        }
        results.push((
            transition.to.clone(),
            AgentMessage {
                id: crate::agent::uuid(),
                role: MessageRole::System,
                from: node_name.to_string(),
                to: Some(transition.to.clone()),
                content: transition
                    .name
                    .clone()
                    .unwrap_or_else(|| "transition".to_string()),
                metadata: None,
            },
        ));
    }
    Ok(results)
}

pub struct FlowExecution {
    pub flow_name: String,
    pub last_node: String,
    pub last_message: Option<AgentMessage>,
}

struct ExecutorRuntime {
    ctx: Arc<FlowContext>,
    tools: Arc<ToolRegistry>,
}

#[async_trait]
impl AgentRuntime for ExecutorRuntime {
    async fn call_tool(
        &self,
        name: &str,
        invocation: crate::tools::ToolInvocation,
    ) -> Result<AgentMessage> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AgentFlowError::ToolNotRegistered(name.to_string()))?;
        let response = tool.call(invocation, &self.ctx).await?;
        Ok(response)
    }

    async fn emit_message(&self, message: AgentMessage) -> Result<()> {
        self.ctx.push_message(message);
        Ok(())
    }
}
