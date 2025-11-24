use crate::agent::{AgentMessage, MessageRole};
use crate::error::Result;
use crate::flow::constants::fields;
use crate::StructuredMessage;
use serde_json::{json, Value};

/// 构建路由消息
pub fn build_route_message(
    target: &str,
    route_label: &str,
    reason: Option<&Value>,
    response: Option<String>,
    payload: &Value,
    agent_name: &str,
) -> Result<AgentMessage> {
    let mut route_message_payload = payload.clone();
    route_message_payload[fields::ROUTE_LABEL] = json!(route_label);
    if let Some(reason) = reason {
        route_message_payload[fields::ROUTE_REASON] = reason.clone();
    }
    if let Some(response) = response {
        route_message_payload[fields::RESPONSE] = json!(response);
    }

    let route_message = StructuredMessage::new(route_message_payload).into_agent_message(
        MessageRole::Agent,
        agent_name,
        Some(target.to_string()),
    )?;

    Ok(route_message)
}
