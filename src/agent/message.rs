use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub role: MessageRole,
    pub from: String,
    pub to: Option<String>,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl AgentMessage {
    pub fn user<T: Into<String>>(content: T) -> Self {
        Self {
            id: uuid(),
            role: MessageRole::User,
            from: "user".into(),
            to: None,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn system<T: Into<String>>(content: T) -> Self {
        Self {
            id: uuid(),
            role: MessageRole::System,
            from: "system".into(),
            to: None,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn tool<T: Into<String>>(name: T, content: T) -> Self {
        Self {
            id: uuid(),
            role: MessageRole::Tool,
            from: name.into(),
            to: None,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn try_decode<T>(&self) -> Result<T, crate::error::AgentFlowError>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str(&self.content)
            .map_err(|e| crate::error::AgentFlowError::Serialization(e.to_string()))
    }

    pub fn from_serialized<T>(
        role: MessageRole,
        from: impl Into<String>,
        to: Option<String>,
        value: &T,
    ) -> Result<Self, crate::error::AgentFlowError>
    where
        T: Serialize,
    {
        let content = serde_json::to_string(value)
            .map_err(|e| crate::error::AgentFlowError::Serialization(e.to_string()))?;
        Ok(Self {
            id: uuid(),
            role,
            from: from.into(),
            to,
            content,
            metadata: None,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    System,
    Assistant,
    Tool,
    Agent,
}

pub fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");
    format!("msg-{}-{}", now.as_secs(), now.subsec_nanos())
}

