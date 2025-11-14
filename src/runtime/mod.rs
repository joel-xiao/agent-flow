use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinSet;
use tracing::{debug, warn};

use crate::agent::{
    self, AgentAction, AgentContext, AgentMessage, AgentRegistry, AgentRuntime, MessageRole,
};
use crate::error::{AgentFlowError, FrameworkError, Result};
use crate::flow::{
    DecisionNode, DecisionPolicy, Flow, FlowNodeKind, JoinNode, JoinStrategy, LoopNode, ToolNode,
};
use crate::state::FlowContext;
use crate::tools::{ToolRegistry, orchestrator::ToolOrchestrator};

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
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(FlowEvent {
            node: self.flow.start.clone(),
            message: initial,
            iterations: 0,
            trace_id: agent::uuid(),
            source: "__start__".to_string(),
        })
        .map_err(|_| AgentFlowError::Other(anyhow!("failed to enqueue initial event")))?;

        let mut join_set: JoinSet<Result<TaskResult>> = JoinSet::new();
        let mut inflight = 0usize;
        let mut finished: Option<FlowExecution> = None;
        let collected_errors: Vec<FrameworkError> = Vec::new();
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

#[derive(Clone)]
struct FlowEvent {
    node: String,
    message: AgentMessage,
    iterations: u32,
    trace_id: String,
    source: String,
}

enum TaskResult {
    Continue,
    Finished(TaskFinished),
}

struct TaskFinished {
    node: String,
    message: Option<AgentMessage>,
}

#[derive(Default)]
struct SharedState {
    join_states: Mutex<HashMap<String, JoinState>>,
    loop_states: Mutex<HashMap<String, LoopState>>,
    started_agents: Mutex<HashSet<String>>,
}

struct JoinState {
    strategy: JoinStrategy,
    expected: HashSet<String>,
    received: HashMap<String, AgentMessage>,
    triggered: bool,
}

impl JoinState {
    fn new(node: JoinNode) -> Self {
        Self {
            strategy: node.strategy,
            expected: node.inbound.into_iter().collect(),
            received: HashMap::new(),
            triggered: false,
        }
    }

    fn record(
        &mut self,
        source: String,
        message: AgentMessage,
    ) -> Option<HashMap<String, AgentMessage>> {
        if self.triggered {
            return None;
        }

        self.received.insert(source.clone(), message);

        match &self.strategy {
            JoinStrategy::All => {
                let required = if self.expected.is_empty() {
                    !self.received.is_empty()
                } else {
                    self.expected
                        .iter()
                        .all(|name| self.received.contains_key(name))
                };
                if required {
                    self.triggered = true;
                    return Some(self.received.clone());
                }
            }
            JoinStrategy::Any => {
                self.triggered = true;
                if let Some(message) = self.received.get(&source).cloned() {
                    let mut map = HashMap::new();
                    map.insert(source, message);
                    return Some(map);
                }
            }
            JoinStrategy::Count(count) => {
                if self.received.len() >= *count {
                    self.triggered = true;
                    return Some(self.received.clone());
                }
            }
        }

        None
    }
}

fn make_join_message(node_name: &str, messages: &HashMap<String, AgentMessage>) -> AgentMessage {
    let aggregated: Vec<_> = messages
        .iter()
        .map(|(source, message)| {
            json!({
                "source": source,
                "id": message.id.clone(),
                "role": format!("{:?}", message.role),
                "content": message.content.clone(),
                "metadata": message.metadata.clone(),
            })
        })
        .collect();

    let payload = json!({
        "join_node": node_name,
        "messages": aggregated,
    });

    AgentMessage {
        id: agent::uuid(),
        role: MessageRole::System,
        from: node_name.to_string(),
        to: None,
        content: payload.to_string(),
        metadata: Some(payload),
    }
}

#[derive(Default)]
struct LoopState {
    iterations: u32,
}

async fn process_event(
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

            {
                let mut started = shared.started_agents.lock().await;
                if started.insert(agent_name.clone()) {
                    agent.on_start(&agent_ctx).await?;
                }
            }

            let action = agent.on_message(event.message.clone(), &agent_ctx).await?;
            if matches!(action, AgentAction::Finish { .. }) {
                agent.on_finish(&agent_ctx).await?;
            }
            handle_action(action, &event, flow, &ctx, &tools, sender).await
        }
        FlowNodeKind::Decision(decision) => {
            handle_decision_node(decision, &node.name, &event, &ctx, sender).await
        }
        FlowNodeKind::Join(join) => {
            handle_join_node(join, &node.name, &event, &ctx, &flow, sender, &shared).await
        }
        FlowNodeKind::Loop(loop_node) => {
            handle_loop_node(loop_node, &node.name, &event, &ctx, sender, &shared).await
        }
        FlowNodeKind::Tool(tool_node) => {
            handle_tool_node(
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

async fn handle_decision_node(
    decision: &DecisionNode,
    node_name: &str,
    event: &FlowEvent,
    ctx: &Arc<FlowContext>,
    sender: mpsc::UnboundedSender<FlowEvent>,
) -> Result<TaskResult> {
    let mut matched: Vec<crate::flow::DecisionBranch> = Vec::new();

    for branch in &decision.branches {
        let passes = if let Some(condition) = &branch.condition {
            (condition)(ctx).await
        } else {
            true
        };

        if passes {
            matched.push(branch.clone());
            if matches!(decision.policy, DecisionPolicy::FirstMatch) {
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

    for branch in matched {
        let metadata = json!({
            "decision": {
                "node": node_name,
                "branch": branch.name,
                "source_message_id": event.message.id.clone(),
                "source_metadata": event.message.metadata.clone(),
            }
        });
        let message = AgentMessage {
            id: agent::uuid(),
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

async fn handle_join_node(
    join: &JoinNode,
    node_name: &str,
    event: &FlowEvent,
    ctx: &Arc<FlowContext>,
    flow: &Arc<Flow>,
    sender: mpsc::UnboundedSender<FlowEvent>,
    shared: &Arc<SharedState>,
) -> Result<TaskResult> {
    let key = format!("{}::{}", event.trace_id, node_name);
    let mut states = shared.join_states.lock().await;
    let state = states
        .entry(key.clone())
        .or_insert_with(|| JoinState::new(join.clone()));
    
    let source_node = event.source.clone();
    if !state.expected.is_empty() && !state.expected.contains(&source_node) {
        drop(states);
        return Ok(TaskResult::Continue);
    }
    
    if let Some(collected) = state.record(source_node, event.message.clone()) {
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

async fn handle_loop_node(
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
        .or_insert_with(|| LoopState::default());

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

async fn handle_tool_node(
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

    let message = orchestrator
        .execute_pipeline(&tool_node.pipeline, ctx)
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
    pub errors: Vec<FrameworkError>,
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
