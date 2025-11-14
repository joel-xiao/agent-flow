use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    AgentMessage, FlowContext, ToolManifest, ToolOrchestrator, ToolPipeline, ToolRegistry,
    ToolStep, ToolStrategy,
};
use anyhow::Result as AnyResult;
use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex;

struct SuccessTool {
    name: &'static str,
    counter: Arc<Mutex<u32>>,
}

#[async_trait]
impl agentflow::Tool for SuccessTool {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn call(
        &self,
        invocation: agentflow::ToolInvocation,
        _ctx: &agentflow::FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        let mut guard = self.counter.lock().await;
        *guard += 1;
        Ok(AgentMessage::tool(
            self.name.to_string(),
            invocation.input.to_string(),
        ))
    }
}

struct FailingTool;

#[async_trait]
impl agentflow::Tool for FailingTool {
    fn name(&self) -> &'static str {
        "failing"
    }

    async fn call(
        &self,
        _invocation: agentflow::ToolInvocation,
        _ctx: &agentflow::FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        Err(agentflow::AgentFlowError::Other(anyhow::anyhow!(
            "intentional failure"
        )))
    }
}

fn dummy_context() -> FlowContext {
    FlowContext::new(Arc::new(MemoryStore::new()))
}

#[tokio::test]
async fn orchestrator_executes_sequential_pipeline() -> AnyResult<()> {
    let counter = Arc::new(Mutex::new(0u32));
    let mut registry = ToolRegistry::new();
    registry.register_with_manifest(
        Arc::new(SuccessTool {
            name: "alpha",
            counter: Arc::clone(&counter),
        }),
        ToolManifest::builder("alpha").build(),
    )?;
    registry.register_with_manifest(
        Arc::new(SuccessTool {
            name: "beta",
            counter: Arc::clone(&counter),
        }),
        ToolManifest::builder("beta").build(),
    )?;

    let pipeline = ToolPipeline::new(
        "seq",
        ToolStrategy::Sequential(vec![
            ToolStep::new("alpha", json!({"value": 1})),
            ToolStep::new("beta", json!({"value": 2})),
        ]),
    );

    let orchestrator = ToolOrchestrator::new(registry);
    let mut orchestrator = orchestrator;
    orchestrator.register_pipeline(pipeline)?;

    let ctx = dummy_context();
    let result = orchestrator.execute_pipeline("seq", &ctx).await?;

    assert_eq!(result.from, "beta");
    assert_eq!(result.role, agentflow::MessageRole::Tool);
    assert_eq!(result.content, r#"{"value":2}"#);
    assert_eq!(*counter.lock().await, 2);
    Ok(())
}

#[tokio::test]
async fn orchestrator_fallback_recovers_from_failure() -> AnyResult<()> {
    let counter = Arc::new(Mutex::new(0u32));
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(FailingTool));
    registry.register_with_manifest(
        Arc::new(SuccessTool {
            name: "beta",
            counter: Arc::clone(&counter),
        }),
        ToolManifest::builder("beta").build(),
    )?;

    let pipeline = ToolPipeline::new(
        "fallback",
        ToolStrategy::Fallback(vec![
            ToolStep::new("failing", json!({"value": 1})).with_retries(1),
            ToolStep::new("beta", json!({"value": 42})),
        ]),
    );

    let mut orchestrator = ToolOrchestrator::new(registry);
    orchestrator.register_pipeline(pipeline)?;

    let ctx = dummy_context();
    let result = orchestrator.execute_pipeline("fallback", &ctx).await?;

    assert_eq!(result.from, "beta");
    assert_eq!(result.content, r#"{"value":42}"#);
    assert_eq!(*counter.lock().await, 1);
    Ok(())
}
