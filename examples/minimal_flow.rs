use std::sync::Arc;

use agentflow::agent::builtin::register_builtin_agent_factories;
use agentflow::autogen::AutogenConfig;
use agentflow::state::MemoryStore;
use agentflow::tools::register_builtin_tool_factories;
use agentflow::{
    AgentFactoryRegistry, AgentMessage, FlowContext, FlowExecutor, FlowRegistry,
    ToolFactoryRegistry, ToolRegistry,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let memory_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(memory_store.clone()));

    let mut agent_factories = AgentFactoryRegistry::new();
    register_builtin_agent_factories(&mut agent_factories);

    let mut tool_factories = ToolFactoryRegistry::new();
    register_builtin_tool_factories(&mut tool_factories);

    let mut agents = agentflow::AgentRegistry::new();
    let mut tools = ToolRegistry::new();
    let mut flow_registry = FlowRegistry::new();
    let autogen = AutogenConfig::from_path("autogen/flows.yaml")?;
    autogen.register_all(
        &agent_factories,
        &mut agents,
        &tool_factories,
        &mut tools,
        &mut flow_registry,
    )?;
    let flow = flow_registry
        .get("code_review")
        .cloned()
        .expect("flow `code_review` not found");

    let executor = FlowExecutor::new(flow, agents, tools);
    let initial = AgentMessage::user("打印 Hello, world!");
    let result = executor.start(ctx, initial).await?;

    println!(
        "Flow `{}` finished at node `{}`",
        result.flow_name, result.last_node
    );

    if let Some(message) = result.last_message {
        println!("Last message: {}", message.content);
    }

    Ok(())
}
