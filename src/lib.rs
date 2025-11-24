pub mod agent;
pub mod cli;
pub mod config;
pub mod error;
pub mod flow;
pub mod llm;
pub mod message;
pub mod plugin;
pub mod runtime;
pub mod schema;
pub mod state;
pub mod tools;
pub mod utils;

pub use agent::{
    register_agent, Agent, AgentAction, AgentContext, AgentFactoryRegistry, AgentInput,
    AgentManifest, AgentManifestBuilder, AgentMessage, AgentOutput, AgentPort, AgentPortSchema,
    AgentRegistry, MessageRole,
};
pub use cli::{load_plugin_manifests, schema_exports, SchemaExportEntry};
pub use error::{AgentFlowError, Result};
pub use flow::config::GraphFlow;
pub use flow::loader::{
    build_flow_from_graph, load_workflow_from_str, load_workflow_from_value, WorkflowBundle,
};
pub use flow::{
    condition_always, condition_from_fn, condition_state_absent, condition_state_equals,
    condition_state_exists, condition_state_not_equals, loop_condition_always,
    loop_condition_from_fn, DecisionBranch, DecisionNode, DecisionPolicy, Flow, FlowBuilder,
    FlowNode, FlowNodeKind, FlowParameter, FlowParameterKind, FlowRegistry, FlowVariable, JoinNode,
    JoinStrategy, LoopContinuation, LoopNode,
};
pub use llm::{DynLlmClient, LlmClient, LlmRequest, LlmResponse, LocalEchoClient};

pub use config::{
    AgentConfig, Condition, DecisionBranchConfig, DecisionNodeConfig, GraphConfig, GraphEdge,
    GraphNode, JoinNodeConfig, LoopNodeConfig, WorkflowConfig,
};
#[cfg(feature = "openai-client")]
pub use llm::{ApiFormat, GenericHttpClient};
pub use message::StructuredMessage;
pub use plugin::{PluginKind, PluginManifest, PluginRegistry};
pub use runtime::{FlowExecution, FlowExecutor};
pub use schema::{register_schema, validate_schema, Schema, SchemaKind, SchemaRegistry};
pub use state::{
    ContextStore, FlowContext, FlowScopeGuard, FlowScopeKind, FlowVariables, SessionContext,
};
pub use tools::{
    orchestrator::{ToolOrchestrator, ToolPipeline, ToolStep, ToolStrategy},
    Tool, ToolFactoryRegistry, ToolInvocation, ToolManifest, ToolManifestBuilder, ToolPort,
    ToolPortSchema, ToolRegistry,
};
pub use utils::{logging, validation};
