use crate::agent::AgentMessage;
use crate::error::{AgentFlowError, Result};
use crate::flow::constants::fields;
use anyhow::anyhow;
use serde_json::Value;

/// 消息解析服务
///
/// 负责从消息内容或历史记录中提取和解析各种数据
pub struct MessageParser;

impl MessageParser {
    /// 从消息内容或历史记录中解析 payload
    ///
    /// 如果当前消息无法解析为 JSON，则从历史记录中查找最后一个有效的 payload
    pub fn parse_payload(message: &AgentMessage, history: &[AgentMessage]) -> Result<Value> {
        match serde_json::from_str::<Value>(&message.content) {
            Ok(payload) => Ok(payload),
            Err(_) => {
                for msg in history.iter().rev() {
                    if let Ok(prev_payload) = serde_json::from_str::<Value>(&msg.content) {
                        return Ok(prev_payload);
                    }
                }
                Err(AgentFlowError::Serialization(format!(
                    "Failed to parse message content as JSON: {}",
                    message.content
                )))
            }
        }
    }

    /// 从消息内容或历史记录中获取 steps
    ///
    /// 使用配置的字段名，如果未配置则使用默认字段名
    pub fn extract_steps(
        payload: &Value,
        history: &[AgentMessage],
        steps_field: Option<&str>,
    ) -> Result<Value> {
        let steps_field_name = steps_field.unwrap_or(fields::STEPS);

        if let Some(steps) = payload.get(steps_field_name) {
            return Ok(steps.clone());
        }

        for msg in history.iter().rev() {
            if let Ok(prev_payload) = serde_json::from_str::<Value>(&msg.content) {
                if let Some(prev_steps) = prev_payload.get(steps_field_name) {
                    return Ok(prev_steps.clone());
                }
            }
        }

        Err(AgentFlowError::Other(anyhow!("Missing steps field")))
    }

    /// 从 payload 或历史记录中提取用户输入
    ///
    /// 使用配置的字段优先级，如果未配置则使用默认顺序
    pub fn extract_user_input(
        payload: &Value,
        history: &[AgentMessage],
        field_priority: Option<&[String]>,
    ) -> Result<String> {
        let default_fields = vec![
            fields::RESPONSE.to_string(),
            fields::RAW.to_string(),
            fields::USER.to_string(),
            fields::GOAL.to_string(),
        ];
        let fields_to_check = field_priority.unwrap_or(&default_fields);

        for field_name in fields_to_check {
            if let Some(input) = payload
                .get(field_name)
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                })
            {
                return Ok(input);
            }
        }

        for msg in history.iter().rev() {
            if let Ok(prev_payload) = serde_json::from_str::<Value>(&msg.content) {
                for field_name in fields_to_check {
                    if let Some(input) = prev_payload
                        .get(field_name)
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        })
                    {
                        return Ok(input);
                    }
                }
            }
        }

        Err(AgentFlowError::Other(anyhow!("Missing user input field")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentMessage, MessageRole};
    use serde_json::json;

    #[test]
    fn test_parse_payload_from_message() {
        let message = AgentMessage {
            role: MessageRole::User,
            content: json!({"user": "test"}).to_string(),
            agent: None,
            target: None,
        };

        let payload = MessageParser::parse_payload(&message, &[]).unwrap();
        assert_eq!(payload["user"], "test");
    }

    #[test]
    fn test_parse_payload_from_history() {
        let message = AgentMessage {
            role: MessageRole::User,
            content: "invalid json".to_string(),
            agent: None,
            target: None,
        };

        let history = vec![AgentMessage {
            role: MessageRole::Agent,
            content: json!({"user": "from history"}).to_string(),
            agent: None,
            target: None,
        }];

        let payload = MessageParser::parse_payload(&message, &history).unwrap();
        assert_eq!(payload["user"], "from history");
    }

    #[test]
    fn test_extract_steps() {
        let payload = json!({
            "steps": ["step1", "step2"]
        });

        let steps = MessageParser::extract_steps(&payload, &[], None).unwrap();
        assert_eq!(steps.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_extract_user_input() {
        let payload = json!({
            "response": "user query"
        });

        let input = MessageParser::extract_user_input(&payload, &[], None).unwrap();
        assert_eq!(input, "user query");
    }
}
