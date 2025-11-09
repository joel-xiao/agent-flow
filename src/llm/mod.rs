use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

#[cfg(feature = "openai-client")]
use crate::error::AgentFlowError;
#[cfg(feature = "openai-client")]
use reqwest;
#[cfg(feature = "openai-client")]
use serde_json::json;
#[cfg(feature = "openai-client")]
use tracing::instrument;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmRequest {
    #[serde(default)]
    pub system: Option<String>,
    pub user: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub metadata: Option<Value>,
}

fn default_temperature() -> f32 {
    0.2
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse>;
}

pub type DynLlmClient = Arc<dyn LlmClient>;

#[derive(Default, Clone)]
pub struct LocalEchoClient;

#[async_trait]
impl LlmClient for LocalEchoClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let mut content = String::new();
        if let Some(system) = &request.system {
            content.push_str(&format!("[System:{}] ", system.trim()));
        }
        content.push_str(&request.user);
        Ok(LlmResponse {
            content,
            metadata: request.metadata,
        })
    }
}

#[cfg(feature = "openai-client")]
pub struct OpenAiClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[cfg(feature = "openai-client")]
impl OpenAiClient {
    pub fn new<S: Into<String>>(api_key: S, model: S) -> Self {
        Self::with_base_url("https://api.openai.com/v1", api_key, model)
    }

    pub fn with_base_url<S1, S2, S3>(base_url: S1, api_key: S2, model: S3) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[cfg(feature = "openai-client")]
#[async_trait]
impl LlmClient for OpenAiClient {
    #[instrument(skip(self))]
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut messages = Vec::new();
        if let Some(system) = &request.system {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }
        messages.push(json!({
            "role": "user",
            "content": request.user
        }));

        let body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": request.temperature,
        });

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "OpenAI request failed with status {}",
                status
            )));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;
        let content = payload["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["content"].as_str())
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("missing message content")))?;

        Ok(LlmResponse {
            content: content.to_string(),
            metadata: Some(payload),
        })
    }
}
