use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::FlowContext;
use crate::agent::{
    Agent, AgentAction, AgentContext, AgentMessage, AgentRegistry, MessageRole, register_agent,
};
#[cfg(feature = "openai-client")]
use crate::LlmRequest;
use crate::error::{AgentFlowError, Result};
use crate::llm::DynLlmClient;
#[cfg(feature = "openai-client")]
use crate::QwenClient;
use crate::state::FlowScopeKind;
use crate::tools::{Tool, ToolRegistry};
use crate::{StructuredMessage, ToolInvocation};

use super::{
    DecisionBranch, DecisionPolicy, Flow, FlowBuilder, FlowParameter, FlowParameterKind,
    FlowVariable, JoinStrategy, LoopContinuation, condition_always, condition_state_absent,
    condition_state_equals, condition_state_exists, condition_state_not_equals,
    loop_condition_always,
};

#[derive(Debug, Deserialize, Clone)]
pub struct GraphParameter {
    pub name: String,
    #[serde(default = "GraphParameter::default_kind")]
    pub kind: String,
    #[serde(default)]
    pub type_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl GraphParameter {
    fn default_kind() -> String {
        "input".into()
    }

    fn into_flow_param(self) -> FlowParameter {
        let mut param = match self.kind.as_str() {
            "input" => FlowParameter::new(self.name.clone(), FlowParameterKind::Input),
            "output" => FlowParameter::new(self.name.clone(), FlowParameterKind::Output),
            "inout" => FlowParameter::new(self.name.clone(), FlowParameterKind::InOut),
            _ => FlowParameter::new(self.name.clone(), FlowParameterKind::Input),
        };
        if let Some(type_name) = self.type_name {
            param = param.with_type(type_name);
        }
        if let Some(desc) = self.description {
            param = param.with_description(desc);
        }
        param
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphVariable {
    pub name: String,
    #[serde(default = "GraphVariable::default_scope")]
    pub scope: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl GraphVariable {
    fn default_scope() -> String {
        "global".into()
    }

    fn into_flow_variable(self) -> FlowVariable {
        let scope = match self.scope.as_str() {
            "global" => FlowScopeKind::Global,
            other => FlowScopeKind::Node(other.into()),
        };
        let mut variable = FlowVariable::new(self.name, scope);
        if let Some(default) = self.default {
            variable = variable.with_default(default);
        }
        if let Some(description) = self.description {
            variable = variable.with_description(description);
        }
        variable
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphTransition {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<GraphCondition>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphCondition {
    Always,
    StateEquals { key: String, value: String },
    StateNotEquals { key: String, value: String },
    StateExists { key: String },
    StateAbsent { key: String },
}

impl GraphCondition {
    fn build(&self) -> crate::flow::TransitionCondition {
        match self {
            GraphCondition::Always => condition_always(),
            GraphCondition::StateEquals { key, value } => {
                condition_state_equals(key.clone(), value.clone())
            }
            GraphCondition::StateNotEquals { key, value } => {
                condition_state_not_equals(key.clone(), value.clone())
            }
            GraphCondition::StateExists { key } => condition_state_exists(key.clone()),
            GraphCondition::StateAbsent { key } => condition_state_absent(key.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphLoopCondition {
    #[serde(default)]
    pub state_equals: Option<LoopConditionStateEquals>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoopConditionStateEquals {
    pub key: String,
    pub value: String,
}

impl GraphLoopCondition {
    fn build(&self) -> LoopContinuation {
        if let Some(state) = &self.state_equals {
            let key = state.key.clone();
            let value = state.value.clone();
            Arc::new(move |ctx| {
                let store = ctx.store();
                let key = key.clone();
                let value = value.clone();
                Box::pin(async move {
                    match store.get(&key).await {
                        Ok(Some(current)) => current == value,
                        _ => false,
                    }
                })
            })
        } else {
            loop_condition_always()
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNode {
    Agent {
        name: String,
        agent: String,
    },
    Decision {
        name: String,
        policy: Option<String>,
        branches: Vec<GraphDecisionBranch>,
    },
    Join {
        name: String,
        strategy: String,
        inbound: Vec<String>,
    },
    Loop {
        name: String,
        entry: String,
        #[serde(default)]
        condition: Option<GraphLoopCondition>,
        #[serde(default)]
        max_iterations: Option<u32>,
        #[serde(default)]
        exit: Option<String>,
    },
    Tool {
        name: String,
        pipeline: String,
    },
    Terminal {
        name: String,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphDecisionBranch {
    pub target: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<GraphCondition>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GraphFlow {
    pub name: String,
    pub start: String,
    #[serde(default)]
    pub parameters: Vec<GraphParameter>,
    #[serde(default)]
    pub variables: Vec<GraphVariable>,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub transitions: Vec<GraphTransition>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AgentDriverKind {
    Echo,
    #[cfg(feature = "openai-client")]
    Qwen,
}

impl Default for AgentDriverKind {
    fn default() -> Self {
        AgentDriverKind::Echo
    }
}

impl AgentDriverKind {
    fn as_str(&self) -> &'static str {
        match self {
            AgentDriverKind::Echo => "echo",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Qwen => "qwen",
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub name: String,
    #[serde(default)]
    pub driver: AgentDriverKind,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ToolDriverKind {
    Echo,
}

impl Default for ToolDriverKind {
    fn default() -> Self {
        ToolDriverKind::Echo
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolConfig {
    pub name: String,
    #[serde(default)]
    pub driver: ToolDriverKind,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    pub flow: GraphFlow,
}

#[derive(Clone)]
struct ConfigDrivenAgent {
    profile: Arc<AgentConfig>,
    name: &'static str,
    #[cfg(feature = "openai-client")]
    llm_client: Option<DynLlmClient>,
}

#[async_trait]
impl Agent for ConfigDrivenAgent {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction> {
        let mut payload: Value = match serde_json::from_str(&message.content) {
            Ok(p) => p,
            Err(_) => {
                let history = ctx.flow().history();
                let mut found_payload = None;
                for msg in history.iter().rev() {
                    if let Ok(prev_payload) = serde_json::from_str::<Value>(&msg.content) {
                        found_payload = Some(prev_payload);
                        break;
                    }
                }
                found_payload.ok_or_else(|| AgentFlowError::Serialization(format!("Failed to parse message content as JSON: {}", message.content)))?
            }
        };

        let mut steps = match payload.get("steps").cloned() {
            Some(s) => s,
            None => {
                let history = ctx.flow().history();
                let mut found_steps = None;
                for msg in history.iter().rev() {
                    if let Ok(prev_payload) = serde_json::from_str::<Value>(&msg.content) {
                        if let Some(prev_steps) = prev_payload.get("steps") {
                            found_steps = Some(prev_steps.clone());
                            break;
                        }
                    }
                }
                found_steps.ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing steps field")))?
            }
        };

        let image_url = payload.get("image_url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let image_base64 = payload.get("image_base64").and_then(|v| v.as_str()).map(|s| s.to_string());
        let image_path = payload.get("image_path").and_then(|v| v.as_str()).map(|s| s.to_string());
        let image_base64_final = if let Some(path) = &image_path {
            use base64::{Engine as _, engine::general_purpose};
            Some(general_purpose::STANDARD.encode(&std::fs::read(path)
                .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Failed to read image file: {}", e)))?))
        } else {
            image_base64
        };

        #[cfg(feature = "openai-client")]
        let response_content = if let Some(llm_client) = &self.llm_client {
            let user_input = payload
                .get("response")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .or_else(|| payload.get("raw").and_then(|v| v.as_str().map(|s| s.to_string())))
                .or_else(|| payload.get("user").and_then(|v| v.as_str().map(|s| s.to_string())))
                .or_else(|| payload.get("goal").and_then(|v| v.as_str().map(|s| s.to_string())))
                .or_else(|| {
                    let history = ctx.flow().history();
                    for msg in history.iter().rev() {
                        if let Ok(prev_payload) = serde_json::from_str::<Value>(&msg.content) {
                            if let Some(response) = prev_payload.get("response").and_then(|v| v.as_str()) {
                                return Some(response.to_string());
                            }
                            if let Some(goal) = prev_payload.get("goal").and_then(|v| v.as_str()) {
                                return Some(goal.to_string());
                            }
                            if let Some(raw) = prev_payload.get("raw").and_then(|v| v.as_str()) {
                                return Some(raw.to_string());
                            }
                        }
                    }
                    None
                })
                .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing user input field")))?;

            let system_prompt = if let Some(role) = &self.profile.role {
                if let Some(prompt) = &self.profile.prompt {
                    format!("You are {}. {}", role, prompt)
                } else {
                    format!("You are {}.", role)
                }
            } else if let Some(prompt) = &self.profile.prompt {
                prompt.clone()
            } else {
                return Err(AgentFlowError::Other(anyhow::anyhow!("Missing role or prompt configuration")));
            };

            let llm_request = LlmRequest {
                system: Some(system_prompt),
                user: user_input.to_string(),
                temperature: 0.7,
                metadata: None,
                image_url: image_url.clone(),
                image_base64: image_base64_final.clone(),
            };

            let role_name = self.profile.role.as_deref().unwrap_or(&self.profile.name);
            println!("\n[{}] Starting response:", role_name);
            print!("  ");
            
            let mut stream = llm_client.complete_stream(llm_request);
            let mut full_response = String::new();
            use futures::StreamExt;
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if !chunk.content.is_empty() {
                            print!("{}", chunk.content);
                            std::io::Write::flush(&mut std::io::stdout())
                                .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Failed to flush stdout: {}", e)))?;
                            full_response.push_str(&chunk.content);
                        }
                        if chunk.done {
                            println!("\n[{}] Response completed\n", role_name);
                            break;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            
            full_response
        } else {
            payload
                .get("raw")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing raw field and no LLM client")))?
        };
        
        #[cfg(not(feature = "openai-client"))]
        let response_content = payload
            .get("raw")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing raw field")))?;

        if let Value::Array(ref mut list) = steps {
            list.push(json!({
                "agent": self.profile.name,
                "intent": self.profile.intent,
                "driver": self.profile.driver.as_str(),
            }));
        }
        payload["steps"] = steps;
        payload["last_agent"] = Value::String(self.profile.name.clone());
        payload["response"] = Value::String(response_content.clone());
        
        if let Some(image_url) = &image_url {
            payload["image_url"] = Value::String(image_url.clone());
        }
        if let Some(img_base64) = &image_base64_final {
            payload["image_base64"] = Value::String(img_base64.clone());
        }
        if let Some(image_path) = &image_path {
            payload["image_path"] = Value::String(image_path.clone());
        }
        
        if let Some(prompt) = &self.profile.prompt {
            payload["prompt"] = Value::String(prompt.clone());
        }
        if let Some(role) = &self.profile.role {
            payload["role"] = Value::String(role.clone());
        }
        if let Some(model) = &self.profile.model {
            payload["model"] = Value::String(model.clone());
        }
        if let Some(metadata) = &self.profile.metadata {
            payload["agent_metadata"] = metadata.clone();
        }

        let message = StructuredMessage::new(payload).into_agent_message(
            MessageRole::Agent,
            &self.profile.name,
            None,
        )?;

        Ok(AgentAction::Continue {
            message: Some(message),
        })
    }
}

#[derive(Clone)]
struct ConfigDrivenTool {
    profile: Arc<ToolConfig>,
    name: &'static str,
}

#[async_trait]
impl Tool for ConfigDrivenTool {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn call(&self, invocation: ToolInvocation, _ctx: &FlowContext) -> Result<AgentMessage> {
        let response = json!({
            "tool": self.profile.name,
            "driver": match self.profile.driver {
                ToolDriverKind::Echo => "echo",
            },
            "input": invocation.input,
        });
        StructuredMessage::new(response).into_agent_message(
            MessageRole::Tool,
            &self.profile.name,
            None,
        )
    }
}

pub struct WorkflowBundle {
    pub flow: Flow,
    pub agents: AgentRegistry,
    pub tools: ToolRegistry,
}

pub fn build_flow_from_graph(graph: &GraphFlow) -> Flow {
    let mut builder = FlowBuilder::new(graph.name.clone());
    builder.set_start(&graph.start);

    for parameter in graph.parameters.clone() {
        builder.with_parameter(parameter.into_flow_param());
    }

    for variable in graph.variables.clone() {
        builder.declare_variable(variable.into_flow_variable());
    }

    for node in &graph.nodes {
        match node {
            GraphNode::Agent { name, agent } => {
                builder.add_agent_node(name, agent);
            }
            GraphNode::Decision {
                name,
                policy,
                branches,
            } => {
                let policy = match policy.as_deref() {
                    Some("all_matches") => DecisionPolicy::AllMatches,
                    _ => DecisionPolicy::FirstMatch,
                };
                let branches = branches
                    .iter()
                    .map(|branch| DecisionBranch {
                        name: branch.name.clone(),
                        condition: branch.condition.as_ref().map(|c| c.build()),
                        target: branch.target.clone(),
                    })
                    .collect::<Vec<_>>();
                builder.add_decision_node(name, policy, branches);
            }
            GraphNode::Join {
                name,
                strategy,
                inbound,
            } => {
                let strategy = match strategy.as_str() {
                    "any" => JoinStrategy::Any,
                    other => {
                        if other.starts_with("count:") {
                            let parts: Vec<_> = other.split(':').collect();
                            let count = parts
                                .get(1)
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1);
                            JoinStrategy::Count(count)
                        } else {
                            JoinStrategy::All
                        }
                    }
                };
                builder.add_join_node(name, strategy, inbound.clone());
            }
            GraphNode::Loop {
                name,
                entry,
                condition,
                max_iterations,
                exit,
            } => {
                let continuation = condition.as_ref().map(|c| c.build());
                builder.add_loop_node(name, entry, continuation, *max_iterations, exit.clone());
            }
            GraphNode::Tool { name, pipeline } => {
                builder.add_tool_node(name, pipeline);
            }
            GraphNode::Terminal { name } => {
                builder.add_terminal_node(name);
            }
        }
    }

    for transition in &graph.transitions {
        if let Some(condition) = &transition.condition {
            builder.connect_if_named(
                &transition.from,
                &transition.to,
                transition.name.clone(),
                condition.build(),
            );
        } else if let Some(name) = &transition.name {
            builder.connect_named(&transition.from, &transition.to, Some(name.clone()));
        } else {
            builder.connect(&transition.from, &transition.to);
        }
    }

    builder.build()
}

pub fn load_workflow_from_value(value: &Value) -> Result<WorkflowBundle> {
    let config: WorkflowConfig = serde_json::from_value(value.clone())
        .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;

    let mut agents = AgentRegistry::new();
    for profile in &config.agents {
        #[cfg(feature = "openai-client")]
        let llm_client: Option<DynLlmClient> = match profile.driver {
            AgentDriverKind::Qwen => {
                let api_key = profile
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("QWEN_API_KEY").ok())
                    .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing QWEN_API_KEY, please provide api_key in config or set environment variable")))?;
                let model = profile
                    .model
                    .clone()
                    .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing model configuration")))?;
                Some(Arc::new(QwenClient::new(api_key, model)))
            }
            AgentDriverKind::Echo => None,
        };
        #[cfg(not(feature = "openai-client"))]
        let _llm_client: Option<DynLlmClient> = None;

        let agent = ConfigDrivenAgent {
            profile: Arc::new(profile.clone()),
            name: Box::leak(profile.name.clone().into_boxed_str()),
            #[cfg(feature = "openai-client")]
            llm_client,
        };
        register_agent(&profile.name, Arc::new(agent), &mut agents);
    }

    let mut tools = ToolRegistry::new();
    for profile in &config.tools {
        let tool = ConfigDrivenTool {
            profile: Arc::new(profile.clone()),
            name: Box::leak(profile.name.clone().into_boxed_str()),
        };
        tools.register(Arc::new(tool));
    }

    let flow = build_flow_from_graph(&config.flow);

    Ok(WorkflowBundle {
        flow,
        agents,
        tools,
    })
}

pub fn load_workflow_from_str(config: &str) -> Result<WorkflowBundle> {
    let value: Value =
        serde_json::from_str(config).map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
    load_workflow_from_value(&value)
}
