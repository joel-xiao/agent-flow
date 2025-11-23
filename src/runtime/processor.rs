use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;
use std::io::{self, Write};

use crate::agent::{AgentAction, AgentContext, AgentRegistry, MessageRole};
use crate::error::{AgentFlowError, Result};
use crate::flow::{Flow, FlowNodeKind};
use crate::state::FlowContext;
use crate::tools::{ToolRegistry, orchestrator::ToolOrchestrator};
use super::types::{FlowEvent, TaskResult, TaskFinished};
use super::state::SharedState;
use super::handlers;
use super::runtime::ExecutorRuntime;

/// 处理单个事件
pub async fn process_event(
    event: FlowEvent,
    flow: Arc<Flow>,
    agents: Arc<AgentRegistry>,
    tools: Arc<ToolRegistry>,
    ctx: Arc<FlowContext>,
    sender: mpsc::UnboundedSender<FlowEvent>,
    max_iterations: u32,
    tool_orchestrator: Option<Arc<ToolOrchestrator>>,
    shared: Arc<SharedState>,
) -> Result<TaskResult> {
    if event.iterations >= max_iterations {
        return Err(AgentFlowError::MaxIterationsExceeded(max_iterations));
    }

    ctx.push_message(event.message.clone());

    let node = flow
        .node(&event.node)
        .ok_or_else(|| AgentFlowError::UnknownNode(event.node.clone()))?;

    // 只在调试模式输出节点执行信息
    let debug_mode = std::env::var("AGENTFLOW_DEBUG").is_ok();
    if debug_mode {
        eprintln!("▶️  正在执行节点: {} ({})", node.name, event.node);
        std::io::stderr().flush().ok();
    }

    match &node.kind {
        FlowNodeKind::Terminal => {
            if debug_mode {
                eprintln!("🏁 到达终端节点: {}", node.name);
                std::io::stderr().flush().ok();
            }
            debug!("Reached terminal node `{}`", node.name);
            return Ok(TaskResult::Finished(TaskFinished {
                node: node.name.clone(),
                message: Some(event.message.clone()),
            }));
        }
        FlowNodeKind::Agent(agent_name) => {
            if debug_mode {
                eprintln!("🤖 开始执行 Agent 节点: {} (agent: {})", node.name, agent_name);
                std::io::stderr().flush().ok();
            }
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

            {
                let mut started = shared.started_agents.lock().await;
                if started.insert(agent_name.clone()) {
                    if debug_mode {
                        eprintln!("  📌 首次启动 Agent: {}", agent_name);
                        io::stderr().flush().ok();
                    }
                    agent.on_start(&agent_ctx).await?;
                }
            }

            let action = agent.on_message(event.message.clone(), &agent_ctx).await?;
            if matches!(action, AgentAction::Finish { .. }) {
                agent.on_finish(&agent_ctx).await?;
            }
            handlers::handle_action(action, &event, flow, &ctx, &tools, sender).await
        }
        FlowNodeKind::Decision(decision) => {
            if debug_mode {
                eprintln!("🔀 执行 Decision 节点: {}", node.name);
                io::stderr().flush().ok();
            }
            handlers::handle_decision_node(decision, &node.name, &event, &ctx, sender).await
        }
        FlowNodeKind::Join(join) => {
            if debug_mode {
                eprintln!("🔗 执行 Join 节点: {}", node.name);
                io::stderr().flush().ok();
            }
            handlers::handle_join_node(join, &node.name, &event, &ctx, &flow, sender, &shared).await
        }
        FlowNodeKind::Loop(loop_node) => {
            handlers::handle_loop_node(loop_node, &node.name, &event, &ctx, sender, &shared).await
        }
        FlowNodeKind::Tool(tool_node) => {
            handlers::handle_tool_node(
                tool_node,
                &node.name,
                &event,
                &ctx,
                Arc::clone(&flow),
                sender,
                tool_orchestrator,
            )
            .await
        }
    }
}

