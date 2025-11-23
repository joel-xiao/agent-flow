use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::agent::{AgentMessage, MessageRole};
use crate::error::{AgentFlowError, Result};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StructuredMessage<T> {
    pub payload: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl<T> StructuredMessage<T> {
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            schema: None,
            metadata: None,
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl<T> StructuredMessage<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn into_agent_message(
        self,
        role: MessageRole,
        from: impl Into<String>,
        to: Option<String>,
    ) -> Result<AgentMessage> {
        let content = serde_json::to_string(&self.payload)
            .map_err(|err| AgentFlowError::Serialization(err.to_string()))?;
        Ok(AgentMessage {
            id: crate::agent::message::uuid(),
            role,
            from: from.into(),
            to,
            content,
            metadata: self
                .metadata
                .or_else(|| self.schema.map(|s| json!({ "schema": s }))),
        })
    }

    pub fn from_agent_message(message: &AgentMessage) -> Result<Self> {
        let payload = serde_json::from_str(&message.content)
            .map_err(|err| AgentFlowError::Serialization(err.to_string()))?;
        let schema = message
            .metadata
            .as_ref()
            .and_then(|meta| meta.get("schema"))
            .and_then(|value| value.as_str().map(|s| s.to_string()));
        Ok(Self {
            payload,
            schema,
            metadata: message.metadata.clone(),
        })
    }
}
