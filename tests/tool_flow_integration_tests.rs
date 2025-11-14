use std::sync::Arc;

use agentflow::agent::AgentRegistry;
use agentflow::state::MemoryStore;
use agentflow::{
    AgentMessage, FlowBuilder, FlowContext, FlowExecutor, ToolManifest, ToolOrchestrator,
    ToolPipeline, ToolRegistry, ToolStep, ToolStrategy,
};
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex;

struct CountingTool {
    counter: Arc<Mutex<u32>>,
}

#[async_trait]
impl agentflow::Tool for CountingTool {
    fn name(&self) -> &'static str {
        "counting"
    }

    async fn call(
        &self,
        invocation: agentflow::ToolInvocation,
        _ctx: &agentflow::FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        let mut count = self.counter.lock().await;
        *count += 1;
        Ok(AgentMessage::tool(
            "counting".to_string(),
            invocation.input.to_string(),
        ))
    }
}

#[tokio::test]
async fn flow_executor_runs_tool_pipeline_node() -> anyhow::Result<()> {
    let counter = Arc::new(Mutex::new(0u32));

    let mut pipeline_registry = ToolRegistry::new();
    pipeline_registry.register_with_manifest(
        Arc::new(CountingTool {
            counter: Arc::clone(&counter),
        }),
        ToolManifest::builder("counting").build(),
    )?;

    let mut orchestrator = ToolOrchestrator::new(pipeline_registry);
    orchestrator.register_pipeline(ToolPipeline::new(
        "pipeline.main",
        ToolStrategy::Sequential(vec![ToolStep::new("counting", json!({"value": 1}))]),
    ))?;
    let orchestrator = Arc::new(orchestrator);

    let mut flow_builder = FlowBuilder::new("tool_flow");
    flow_builder
        .add_tool_node("pipeline", "pipeline.main")
        .add_terminal_node("finish")
        .set_start("pipeline")
        .connect("pipeline", "finish");

    let flow = flow_builder.build();
    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, AgentRegistry::new(), ToolRegistry::new())
        .with_tool_orchestrator(Arc::clone(&orchestrator));

    let result = executor.start(ctx, AgentMessage::system("start")).await?;

    assert_eq!(result.flow_name, "tool_flow");
    assert_eq!(result.last_node, "finish");
    assert_eq!(*counter.lock().await, 1);
    Ok(())
}
