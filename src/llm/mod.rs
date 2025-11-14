use std::sync::Arc;
use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "openai-client")]
use serde_json::json;

use crate::error::{Result, AgentFlowError};

#[cfg(feature = "openai-client")]
use reqwest;
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
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_base64: Option<String>,
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

#[derive(Clone, Debug)]
pub struct LlmStreamChunk {
    pub content: String,
    pub done: bool,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse>;
    
    fn complete_stream(&self, request: LlmRequest) -> LlmStream {
        let request = Arc::new(request);
        let client = self.clone_dyn();
        
        Box::pin(futures::stream::unfold(
            (request, client, None::<String>, 0usize),
            move |(req, client, mut full_content, mut pos)| async move {
                if full_content.is_none() {
                    match client.complete((*req).clone()).await {
                        Ok(response) => {
                            full_content = Some(response.content);
                            pos = 0;
                        }
                        Err(e) => {
                            return Some((Err(e), (req, client, full_content, pos)));
                        }
                    }
                }
                
                let content = match full_content.as_ref() {
                    Some(c) => c,
                    None => return Some((Err(AgentFlowError::Other(anyhow::anyhow!("Stream response content is empty")).into()), (req, client, full_content, pos))),
                };
                if pos < content.len() {
                    let char_start = pos;
                    let ch = match content[char_start..].chars().next() {
                        Some(c) => c,
                        None => return Some((Err(AgentFlowError::Other(anyhow::anyhow!("Character parsing failed")).into()), (req, client, full_content, pos))),
                    };
                    pos += ch.len_utf8();
                    
                    Some((
                        Ok(LlmStreamChunk {
                            content: ch.to_string(),
                            done: false,
                        }),
                        (req, client, full_content, pos),
                    ))
                } else {
                    None
                }
            },
        ).chain(futures::stream::once(async move {
            Ok(LlmStreamChunk {
                content: String::new(),
                done: true,
            })
        })))
    }
    
    fn clone_dyn(&self) -> Arc<dyn LlmClient>;
}

pub type DynLlmClient = Arc<dyn LlmClient>;

#[derive(Default, Clone)]
pub struct LocalEchoClient;

#[async_trait]
impl LlmClient for LocalEchoClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: format!("[Echo] {}", request.user),
            metadata: None,
        })
    }

    fn clone_dyn(&self) -> Arc<dyn LlmClient> {
        Arc::new(LocalEchoClient)
    }
}

#[cfg(feature = "openai-client")]
#[derive(Clone)]
pub struct OpenAiClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[cfg(feature = "openai-client")]
impl OpenAiClient {
    pub fn new<S: Into<String>>(api_key: S, model: S) -> Self {
        Self::with_base_url(
            "https://api.openai.com/v1/chat/completions",
            api_key,
            model,
        )
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
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("OpenAI request error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "OpenAI request failed with status {}: {}",
                status,
                error_text
            )));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("OpenAI response parse error: {}", e)))?;

        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                AgentFlowError::Other(anyhow::anyhow!(
                    "missing content in OpenAI response: {}",
                    serde_json::to_string(&payload).unwrap_or_default()
                ))
            })?;

        Ok(LlmResponse {
            content: content.to_string(),
            metadata: Some(payload),
        })
    }

    fn clone_dyn(&self) -> Arc<dyn LlmClient> {
        Arc::new(OpenAiClient {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
        })
    }
}

#[cfg(feature = "openai-client")]
pub struct QwenClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[cfg(feature = "openai-client")]
impl QwenClient {
    pub fn new<S: Into<String>>(api_key: S, model: S) -> Self {
        Self::with_base_url(
            "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation",
            api_key,
            model,
        )
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
impl LlmClient for QwenClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let is_vision_model = self.model.contains("vl") || self.model.contains("vision");
        let endpoint = if is_vision_model {
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
        } else {
            &self.base_url
        };

        let mut messages = Vec::new();
        if let Some(system) = &request.system {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        let mut user_content = Vec::new();
        user_content.push(json!({
            "type": "text",
            "text": request.user
        }));

        if let Some(image_url) = &request.image_url {
            user_content.push(json!({
                "type": "image_url",
                "image_url": {
                    "url": image_url
                }
            }));
        } else if let Some(image_base64) = &request.image_base64 {
            user_content.push(json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:image/jpeg;base64,{}", image_base64)
                }
            }));
        }

        messages.push(json!({
            "role": "user",
            "content": user_content
        }));

        let body = if is_vision_model {
            json!({
                "model": self.model,
                "messages": messages,
                "temperature": request.temperature,
                "max_tokens": 2000
            })
        } else {
            json!({
                "model": self.model,
                "input": {
                    "messages": messages
                },
                "parameters": {
                    "temperature": request.temperature,
                    "max_tokens": 2000
                }
            })
        };

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Qwen request error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "Qwen request failed with status {}: {}",
                status,
                error_text
            )));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Qwen response parse error: {}", e)))?;

        let content = payload["output"]["text"]
            .as_str()
            .or_else(|| {
                payload["output"]["choices"]
                    .get(0)
                    .and_then(|choice| choice["message"]["content"].as_str())
            })
            .or_else(|| {
                payload["choices"]
                    .get(0)
                    .and_then(|choice| choice["message"]["content"].as_str())
            })
            .ok_or_else(|| {
                AgentFlowError::Other(anyhow::anyhow!(
                    "missing content in Qwen response: {}",
                    serde_json::to_string(&payload).unwrap_or_default()
                ))
            })?;

        Ok(LlmResponse {
            content: content.to_string(),
            metadata: Some(payload),
        })
    }

    fn clone_dyn(&self) -> Arc<dyn LlmClient> {
        Arc::new(QwenClient {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
        })
    }
}
