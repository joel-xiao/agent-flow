use agentflow::{ToolManifest, ToolPort, ToolPortSchema, ToolRegistry};
use serde_json::json;
use std::sync::Arc;

struct SimpleTool;

#[async_trait::async_trait]
impl agentflow::Tool for SimpleTool {
    fn name(&self) -> &'static str {
        "simple"
    }

    async fn call(
        &self,
        invocation: agentflow::ToolInvocation,
        _ctx: &agentflow::FlowContext,
    ) -> agentflow::Result<agentflow::AgentMessage> {
        let payload = invocation.input.to_string();
        Ok(agentflow::AgentMessage::tool("simple".to_string(), payload))
    }
}

#[test]
fn tool_manifest_builder_populates_fields() {
    let manifest = ToolManifest::builder("simple")
        .description("echo-like tool")
        .input(
            ToolPort::new("text")
                .with_description("input payload")
                .with_schema(
                    ToolPortSchema::new()
                        .with_type("String")
                        .with_format("text"),
                ),
        )
        .output(
            ToolPort::new("result").with_schema(
                ToolPortSchema::new()
                    .with_type("AgentMessage")
                    .with_json_schema(json!({"type":"string"})),
            ),
        )
        .capability("echo")
        .permission("fs.read")
        .resource("default")
        .build();

    assert_eq!(manifest.name, "simple");
    assert_eq!(manifest.description.as_deref(), Some("echo-like tool"));
    assert_eq!(manifest.inputs.len(), 1);
    assert_eq!(manifest.outputs.len(), 1);
    assert_eq!(manifest.capabilities, vec!["echo"]);
    assert_eq!(manifest.permissions, vec!["fs.read"]);
    assert_eq!(manifest.resources, vec!["default"]);
}

#[test]
fn tool_registry_registers_and_returns_manifest() -> agentflow::Result<()> {
    let tool: Arc<dyn agentflow::Tool> = Arc::new(SimpleTool);
    let manifest = ToolManifest::builder("simple")
        .description("echo-like tool")
        .build();

    let mut registry = ToolRegistry::new();
    registry.register_with_manifest(Arc::clone(&tool), manifest.clone())?;

    let fetched_tool = registry.get("simple").expect("tool registered");
    assert_eq!(fetched_tool.name(), "simple");
    let fetched_manifest = registry.manifest("simple").expect("manifest registered");
    assert_eq!(fetched_manifest.description, manifest.description);
    Ok(())
}

#[test]
fn tool_registry_manifest_mismatch_returns_error() {
    let tool: Arc<dyn agentflow::Tool> = Arc::new(SimpleTool);
    let manifest = ToolManifest::builder("other").build();

    let mut registry = ToolRegistry::new();
    let err = registry
        .register_with_manifest(tool, manifest)
        .expect_err("should fail");
    match err {
        agentflow::AgentFlowError::ManifestMismatch { kind, name } => {
            assert_eq!(kind, "tool");
            assert_eq!(name, "simple");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
