use agentflow::{AgentManifest, AgentPort, AgentPortSchema};
use serde_json::json;

#[test]
fn agent_manifest_builder_populates_fields() {
    let manifest = AgentManifest::builder("worker")
        .description("demo agent")
        .input(
            AgentPort::new("request")
                .with_description("incoming payload")
                .with_schema(
                    AgentPortSchema::new()
                        .with_type("serde_json::Value")
                        .with_format("json"),
                ),
        )
        .output(
            AgentPort::new("response").with_schema(
                AgentPortSchema::new()
                    .with_type("agentflow::AgentMessage")
                    .with_json_schema(json!({ "type": "object" })),
            ),
        )
        .tool("echo")
        .capability("stream")
        .build();

    assert_eq!(manifest.name, "worker");
    assert_eq!(manifest.description.as_deref(), Some("demo agent"));
    assert_eq!(manifest.inputs.len(), 1);
    assert_eq!(manifest.outputs.len(), 1);
    assert_eq!(manifest.tools, vec!["echo"]);
    assert_eq!(manifest.capabilities, vec!["stream"]);

    let input = &manifest.inputs[0];
    assert_eq!(input.name, "request");
    assert_eq!(input.description.as_deref(), Some("incoming payload"));
    let input_schema = input.schema.as_ref().expect("input schema");
    assert_eq!(input_schema.type_name.as_deref(), Some("serde_json::Value"));
    assert_eq!(input_schema.format.as_deref(), Some("json"));

    let output_schema = manifest.outputs[0].schema.as_ref().expect("output schema");
    assert_eq!(
        output_schema.type_name.as_deref(),
        Some("agentflow::AgentMessage")
    );
    assert_eq!(
        output_schema.json_schema.as_ref(),
        Some(&json!({ "type": "object" }))
    );
}
