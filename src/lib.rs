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

pub use agent::{
    Agent, AgentAction, AgentContext, AgentFactoryRegistry, AgentInput, AgentManifest,
    AgentManifestBuilder, AgentMessage, AgentOutput, AgentPort, AgentPortSchema, AgentRegistry,
    MessageRole, register_agent,
};
pub use cli::{SchemaExportEntry, load_plugin_manifests, schema_exports};
pub use error::{AgentFlowError, Result};
pub use flow::loader::{
    WorkflowBundle, build_flow_from_graph, load_workflow_from_str,
    load_workflow_from_value,
};
pub use flow::config::GraphFlow;
pub use flow::{
    DecisionBranch, DecisionNode, DecisionPolicy, Flow, FlowBuilder, FlowNode, FlowNodeKind,
    FlowParameter, FlowParameterKind, FlowRegistry, FlowVariable, JoinNode, JoinStrategy,
    LoopContinuation, LoopNode, condition_always, condition_from_fn, condition_state_absent,
    condition_state_equals, condition_state_exists, condition_state_not_equals,
    loop_condition_always, loop_condition_from_fn,
};
pub use llm::{DynLlmClient, LlmClient, LlmRequest, LlmResponse, LocalEchoClient};

#[cfg(feature = "openai-client")]
pub use llm::{GenericHttpClient, ApiFormat};
pub use message::StructuredMessage;
pub use plugin::{PluginKind, PluginManifest, PluginRegistry};
pub use runtime::{FlowExecution, FlowExecutor};
pub use schema::{Schema, SchemaKind, SchemaRegistry, register_schema, validate_schema};
pub use state::{
    ContextStore, FlowContext, FlowScopeGuard, FlowScopeKind, FlowVariables, SessionContext,
};
pub use tools::{
    Tool, ToolFactoryRegistry, ToolInvocation, ToolManifest, ToolManifestBuilder, ToolPort,
    ToolPortSchema, ToolRegistry,
    orchestrator::{ToolOrchestrator, ToolPipeline, ToolStep, ToolStrategy},
};
pub use config::{
    GraphConfig, GraphNode, GraphEdge, Condition,
    ServiceConfig, AgentConfig, AgentNodeConfig, DecisionNodeConfig, 
    DecisionBranchConfig, JoinNodeConfig, LoopNodeConfig, WorkflowConfig,
};
