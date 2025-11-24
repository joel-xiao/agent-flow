use anyhow::anyhow;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;

use super::state::{make_join_message, SharedState};
use super::types::{FlowEvent, TaskFinished, TaskResult};
use crate::agent::{AgentAction, AgentMessage, MessageRole};
use crate::error::{AgentFlowError, Result};
use crate::flow::{DecisionNode, Flow, JoinNode, LoopNode, ToolNode};
use crate::state::FlowContext;
use crate::tools::{orchestrator::ToolOrchestrator, ToolRegistry};

/// 处理 Agent Action
pub async fn handle_action(
    action: AgentAction,
    event: &FlowEvent,
    flow: Arc<Flow>,
    ctx: &Arc<FlowContext>,
    tools: &Arc<ToolRegistry>,
    sender: mpsc::UnboundedSender<FlowEvent>,
) -> Result<TaskResult> {
    match action {
        AgentAction::Next { target, message } => {
            enqueue_event(
                sender,
                target,
                message,
                event.iterations + 1,
                &event.trace_id,
                &event.node,
            )?;
            Ok(TaskResult::Continue)
        }
        AgentAction::Branch { branches } => {
            let debug_mode = std::env::var("AGENTFLOW_DEBUG").is_ok();
            if debug_mode {
                use std::io::{self, Write};
                eprintln!(
                    "  🌿 Agent 返回 Branch action，路由到 {} 个目标节点",
                    branches.len()
                );
                for (target, _) in &branches {
                    if flow.node(target).is_some() {
                        eprintln!("    ➡️  路由到节点: {}", target);
                    }
                }
                io::stderr().flush().ok();
            }
            let mut dispatched = false;
            for (target, message) in branches {
                if flow.node(&target).is_some() {
                    enqueue_event(
                        sender.clone(),
                        target,
                        message,
                        event.iterations + 1,
                        &event.trace_id,
                        &event.node,
                    )?;
                    dispatched = true;
                }
            }
            if dispatched {
                Ok(TaskResult::Continue)
            } else {
                if debug_mode {
                    use std::io::{self, Write};
                    eprintln!("  ❌ 没有找到有效的分支，停止工作流");
                    io::stderr().flush().ok();
                }
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
            let runtime_handle = super::runtime::ExecutorRuntime {
                ctx: Arc::clone(ctx),
                tools: Arc::clone(tools),
            };
            let tool_message =
                <super::runtime::ExecutorRuntime as crate::agent::AgentRuntime>::call_tool(
                    &runtime_handle,
                    &tool,
                    invocation,
                )
                .await?;
            ctx.push_message(tool_message.clone());

            if let Some(target) = on_complete {
                enqueue_event(
                    sender,
                    target,
                    tool_message,
                    event.iterations + 1,
                    &event.trace_id,
                    &event.node,
                )?;
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
            let debug_mode = std::env::var("AGENTFLOW_DEBUG").is_ok();
            let transitions = next_from_flow(&event.node, &flow, ctx).await?;
            if transitions.is_empty() {
                if debug_mode {
                    use std::io::{self, Write};
                    eprintln!("  ⚠️  节点 {} 没有后续节点，工作流结束", event.node);
                    io::stderr().flush().ok();
                }
                return Ok(TaskResult::Finished(TaskFinished {
                    node: event.node.clone(),
                    message,
                }));
            }

            if debug_mode {
                use std::io::{self, Write};
                eprintln!(
                    "  ➡️  节点 {} 有 {} 个后续节点",
                    event.node,
                    transitions.len()
                );
                for (target, _) in &transitions {
                    eprintln!("    → 路由到: {}", target);
                }
                io::stderr().flush().ok();
            }
            for (target, default_message) in transitions {
                let to_send = message
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| default_message.clone());
                enqueue_event(
                    sender.clone(),
                    target,
                    to_send,
                    event.iterations + 1,
                    &event.trace_id,
                    &event.node,
                )?;
            }
            Ok(TaskResult::Continue)
        }
    }
}

/// 处理 Decision 节点
pub async fn handle_decision_node(
    decision: &DecisionNode,
    node_name: &str,
    event: &FlowEvent,
    ctx: &Arc<FlowContext>,
    sender: mpsc::UnboundedSender<FlowEvent>,
) -> Result<TaskResult> {
    let debug_mode = std::env::var("AGENTFLOW_DEBUG").is_ok();
    if debug_mode {
        eprintln!("  🔀 Decision 节点: {} 开始评估分支条件", node_name);
        io::stderr().flush().ok();
    }
    let mut matched: Vec<crate::flow::DecisionBranch> = Vec::new();

    for branch in &decision.branches {
        let passes = if let Some(condition) = &branch.condition {
            let result = (condition)(ctx).await;
            if debug_mode {
                eprintln!(
                    "    🔍 检查分支条件: {:?} - {}",
                    branch.name,
                    if result { "✅" } else { "❌" }
                );
                io::stderr().flush().ok();
            }
            result
        } else {
            true
        };

        if passes {
            matched.push(branch.clone());
            if matches!(decision.policy, crate::flow::DecisionPolicy::FirstMatch) {
                break;
            }
        }
    }

    if matched.is_empty() {
        warn!(node = %node_name, "Decision node had no matching branches");
        return Err(AgentFlowError::DecisionNoMatch {
            node: node_name.to_string(),
        });
    }

    if debug_mode {
        eprintln!("  ✅ Decision 节点匹配到 {} 个分支", matched.len());
        for branch in &matched {
            eprintln!(
                "    ➡️  路由到: {} (分支: {:?})",
                branch.target, branch.name
            );
        }
        io::stderr().flush().ok();
    }
    for branch in matched {
        let metadata = serde_json::json!({
            "decision": {
                "node": node_name,
                "branch": branch.name,
                "source_message_id": event.message.id.clone(),
                "source_metadata": event.message.metadata.clone(),
            }
        });
        let message = AgentMessage {
            id: crate::agent::message::uuid(),
            role: event.message.role.clone(),
            from: node_name.to_string(),
            to: Some(branch.target.clone()),
            content: event.message.content.clone(),
            metadata: Some(metadata),
        };

        enqueue_event(
            sender.clone(),
            branch.target.clone(),
            message,
            event.iterations + 1,
            &event.trace_id,
            node_name,
        )?;
    }

    Ok(TaskResult::Continue)
}

/// 处理 Join 节点
pub async fn handle_join_node(
    join: &JoinNode,
    node_name: &str,
    event: &FlowEvent,
    ctx: &Arc<FlowContext>,
    flow: &Arc<Flow>,
    sender: mpsc::UnboundedSender<FlowEvent>,
    shared: &Arc<SharedState>,
) -> Result<TaskResult> {
    let debug_mode = std::env::var("AGENTFLOW_DEBUG").is_ok();
    let key = format!("{}::{}", event.trace_id, node_name);

    if debug_mode {
        use std::io::{self, Write};
        eprintln!(
            "  🔗 Join 节点: {} 收到消息 (来源: {})",
            node_name, event.source
        );
        io::stderr().flush().ok();
    }

    let mut states = shared.join_states.lock().await;
    let state = states.entry(key.clone()).or_insert_with(|| {
        if debug_mode {
            use std::io::{self, Write};
            eprintln!(
                "    📋 初始化 Join 状态，等待 {} 个节点",
                join.inbound.len()
            );
            for inbound in &join.inbound {
                eprintln!("      - {}", inbound);
            }
            io::stderr().flush().ok();
        }
        crate::runtime::state::JoinState::new(join.clone())
    });

    let source_node = event.source.clone();

    if !state.expected.is_empty() && !state.expected.contains(&source_node) {
        if debug_mode {
            use std::io::{self, Write};
            eprintln!("    ⚠️  来源节点 {} 不在预期列表中，忽略", source_node);
            io::stderr().flush().ok();
        }
        drop(states);
        return Ok(TaskResult::Continue);
    }

    if debug_mode {
        use std::io::{self, Write};
        eprintln!("    ✅ 来源节点 {} 匹配，记录消息", source_node);
        io::stderr().flush().ok();
    }
    if let Some(collected) = state.record(source_node, event.message.clone()) {
        if debug_mode {
            use std::io::{self, Write};
            eprintln!("    🎉 Join 节点已收集到所有预期消息，继续执行");
            io::stderr().flush().ok();
        }
        let aggregated = make_join_message(node_name, &collected);
        states.remove(&key);
        drop(states);

        let transitions = next_from_flow(node_name, flow, ctx).await?;
        if transitions.is_empty() {
            return Ok(TaskResult::Finished(TaskFinished {
                node: node_name.to_string(),
                message: Some(aggregated),
            }));
        }

        for (target, default_message) in transitions {
            let to_send = AgentMessage {
                to: default_message.to,
                ..aggregated.clone()
            };
            enqueue_event(
                sender.clone(),
                target,
                to_send,
                event.iterations + 1,
                &event.trace_id,
                node_name,
            )?;
        }
        Ok(TaskResult::Continue)
    } else {
        Ok(TaskResult::Continue)
    }
}

/// 处理 Loop 节点
pub async fn handle_loop_node(
    loop_node: &LoopNode,
    node_name: &str,
    event: &FlowEvent,
    ctx: &Arc<FlowContext>,
    sender: mpsc::UnboundedSender<FlowEvent>,
    shared: &Arc<SharedState>,
) -> Result<TaskResult> {
    let key = format!("{}::{}", event.trace_id, node_name);
    let mut loops = shared.loop_states.lock().await;
    let state = loops
        .entry(key.clone())
        .or_insert_with(|| crate::runtime::state::LoopState::default());

    if let Some(max) = loop_node.max_iterations {
        if state.iterations >= max {
            loops.remove(&key);
            return Err(AgentFlowError::LoopBoundExceeded {
                node: node_name.to_string(),
                max,
            });
        }
    }

    if let Some(condition) = &loop_node.condition {
        let continue_loop = (condition)(ctx).await;
        if !continue_loop {
            loops.remove(&key);
            if let Some(exit) = &loop_node.exit {
                enqueue_event(
                    sender,
                    exit.clone(),
                    event.message.clone(),
                    event.iterations + 1,
                    &event.trace_id,
                    node_name,
                )?;
                return Ok(TaskResult::Continue);
            } else {
                return Ok(TaskResult::Finished(TaskFinished {
                    node: node_name.to_string(),
                    message: Some(event.message.clone()),
                }));
            }
        }
    }

    state.iterations += 1;
    drop(loops);

    enqueue_event(
        sender,
        loop_node.entry.clone(),
        event.message.clone(),
        event.iterations + 1,
        &event.trace_id,
        node_name,
    )?;
    Ok(TaskResult::Continue)
}

/// 处理 Tool 节点
pub async fn handle_tool_node(
    tool_node: &ToolNode,
    node_name: &str,
    event: &FlowEvent,
    ctx: &Arc<FlowContext>,
    flow: Arc<Flow>,
    sender: mpsc::UnboundedSender<FlowEvent>,
    tool_orchestrator: Option<Arc<ToolOrchestrator>>,
) -> Result<TaskResult> {
    let orchestrator = tool_orchestrator
        .ok_or_else(|| AgentFlowError::Other(anyhow!("tool orchestrator not configured")))?;

    let params = tool_node.params.clone().unwrap_or_else(|| serde_json::json!({}));

    let message = orchestrator
        .execute_pipeline_with_params(&tool_node.pipeline, params, ctx)
        .await?;

    ctx.push_message(message.clone());

    let transitions = next_from_flow(node_name, &flow, ctx).await?;
    if transitions.is_empty() {
        return Ok(TaskResult::Finished(TaskFinished {
            node: node_name.to_string(),
            message: Some(message),
        }));
    }

    for (target, default_message) in transitions {
        let mut to_send = message.clone();
        if to_send.to.is_none() {
            to_send.to = default_message.to;
        }
        enqueue_event(
            sender.clone(),
            target,
            to_send,
            event.iterations + 1,
            &event.trace_id,
            node_name,
        )?;
    }
    Ok(TaskResult::Continue)
}

/// 入队事件
fn enqueue_event(
    sender: mpsc::UnboundedSender<FlowEvent>,
    target: String,
    message: AgentMessage,
    iterations: u32,
    trace_id: &str,
    source: &str,
) -> Result<()> {
    if sender
        .send(FlowEvent {
            node: target,
            message,
            iterations,
            trace_id: trace_id.to_string(),
            source: source.to_string(),
        })
        .is_err()
    {
        warn!("scheduler channel closed before event could be enqueued");
    }
    Ok(())
}

/// 从 Flow 获取下一个转换
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
                id: crate::agent::message::uuid(),
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
