use agentflow::{AgentInput, AgentMessage, AgentOutput, MessageRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Request {
    prompt: String,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Response {
    answer: String,
}

#[test]
fn agent_input_parses_json_payload() {
    let original = Request {
        prompt: "ping".to_string(),
        temperature: 0.2,
    };
    let message = AgentMessage::from_serialized(MessageRole::User, "user", None, &original)
        .expect("serialize request");

    let typed = AgentInput::<Request>::try_from_message(message.clone()).expect("parse");
    assert_eq!(typed.value, original);
    assert_eq!(typed.message.id, message.id);

    let from_method: Request = message.try_decode().expect("decode via helper");
    assert_eq!(from_method, original);
}

#[test]
fn agent_output_serializes_payload() {
    let response = Response {
        answer: "pong".to_string(),
    };
    let output = AgentOutput {
        role: MessageRole::Assistant,
        from: "assistant".to_string(),
        to: Some("user".to_string()),
        value: response.clone(),
        metadata: None,
    };
    let message = output.into_message().expect("serialize response");
    assert_eq!(message.role, MessageRole::Assistant);
    assert_eq!(message.to.as_deref(), Some("user"));

    let decoded: Response = message.try_decode().expect("decode response");
    assert_eq!(decoded, response);
}

#[test]
fn agent_input_returns_error_on_invalid_json() {
    let message = AgentMessage {
        id: "raw".to_string(),
        role: MessageRole::User,
        from: "user".to_string(),
        to: None,
        content: "not-json".to_string(),
        metadata: None,
    };
    let result = AgentInput::<Request>::try_from_message(message);
    assert!(result.is_err());
}
