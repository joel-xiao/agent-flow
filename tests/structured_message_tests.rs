use agentflow::{AgentMessage, MessageRole, StructuredMessage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Payload {
    value: String,
    count: u32,
}

#[test]
fn structured_message_roundtrip() {
    let message = StructuredMessage::new(Payload {
        value: "hello".to_string(),
        count: 3,
    })
    .with_schema("payload.schema")
    .into_agent_message(MessageRole::Tool, "tool-a", Some("agent".to_string()))
    .expect("serialize");

    assert_eq!(message.role, MessageRole::Tool);
    assert_eq!(message.from, "tool-a");

    let parsed: StructuredMessage<Payload> =
        StructuredMessage::from_agent_message(&message).expect("deserialize");
    assert_eq!(parsed.payload.count, 3);
    assert_eq!(parsed.schema.as_deref(), Some("payload.schema"));
}

#[test]
fn structured_message_invalid_payload() {
    let invalid = AgentMessage::tool("tool", "not-json");
    let result: Result<StructuredMessage<Payload>, _> =
        StructuredMessage::from_agent_message(&invalid);
    assert!(result.is_err());
}
