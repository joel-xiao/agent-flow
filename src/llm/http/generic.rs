use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use serde_json::Value;

#[cfg(feature = "openai-client")]
use reqwest;
#[cfg(feature = "openai-client")]
use tracing::instrument;

use crate::error::{Result, AgentFlowError};
use crate::llm::client::{LlmClient, DynLlmClient, LlmStream};
use crate::llm::types::{LlmRequest, LlmResponse, LlmStreamChunk, ApiFormat};
use super::stream::SseParser;
use futures::StreamExt;
use anyhow::anyhow;

#[cfg(feature = "openai-client")]
#[derive(Clone)]
pub struct GenericHttpClient {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
    format: ApiFormat,
    auth_header: Option<String>,
}

#[cfg(feature = "openai-client")]
impl GenericHttpClient {
    pub fn new<S1, S2, S3>(endpoint: S1, api_key: S2, model: S3, format: ApiFormat) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            format,
            auth_header: None,
        }
    }

    pub fn with_auth_header<S1, S2, S3, S4>(endpoint: S1, api_key: S2, model: S3, format: ApiFormat, auth_header: S4) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
        S4: Into<String>,
    {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            format,
            auth_header: Some(auth_header.into()),
        }
    }
}

#[cfg(feature = "openai-client")]
#[async_trait]
impl LlmClient for GenericHttpClient {
    #[instrument(skip(self))]
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let mut messages = Vec::new();
        if let Some(system) = &request.system {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        let mut user_content: Value = json!(request.user);
        if request.image_url.is_some() || request.image_base64.is_some() {
            let mut content_parts = Vec::new();
            content_parts.push(json!({
                "type": "text",
                "text": request.user
            }));
            if let Some(image_url) = &request.image_url {
                content_parts.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": image_url
                    }
                }));
            } else if let Some(image_base64) = &request.image_base64 {
                content_parts.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/jpeg;base64,{}", image_base64)
                    }
                }));
            }
            user_content = json!(content_parts);
        }

        messages.push(json!({
            "role": "user",
            "content": user_content
        }));

        let body = match &self.format {
            ApiFormat::OpenAI => {
                let mut body = json!({
                    "model": self.model,
                    "messages": messages,
                });
                if self.endpoint.contains("bigmodel.cn") {
                    // BigModel API: temperature should be between 0 and 2, round to 2 decimal places
                    // Use format! to ensure proper decimal representation
                    let temp = (request.temperature * 100.0).round() / 100.0;
                    if temp > 0.0 && temp <= 2.0 {
                        // Format as string then parse to ensure clean decimal representation
                        let temp_str = format!("{:.2}", temp);
                        if let Ok(temp_val) = temp_str.parse::<f64>() {
                            body["temperature"] = json!(temp_val);
                        }
                    }
                    // max_tokens is optional for BigModel, removed to avoid parameter errors
                } else {
                    body["temperature"] = json!(request.temperature);
                }
                body
            }
            ApiFormat::QwenVision => {
                json!({
                    "model": self.model,
                    "messages": messages,
                    "temperature": request.temperature,
                    "max_tokens": 2000
                })
            }
            ApiFormat::Qwen => {
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
            }
        };

        let auth_value = self.auth_header.as_ref()
            .map(|h| format!("{} {}", h, self.api_key))
            .unwrap_or_else(|| format!("Bearer {}", self.api_key));

        // 构建完整的 endpoint URL
        // 如果 endpoint 不包含路径，根据 endpoint 格式和 API 格式添加默认路径
        let full_endpoint = if self.endpoint.contains("/chat/completions") 
            || self.endpoint.contains("/services/")
            || self.endpoint.contains("/generation") {
            // 已经包含完整路径
            self.endpoint.clone()
        } else {
            // 检查是否是 compatible-mode（OpenAI 兼容模式）
            if self.endpoint.contains("compatible-mode") {
                // OpenAI 兼容模式统一使用 /chat/completions
                format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
            } else {
                // 根据 API 格式添加路径
                match &self.format {
                    ApiFormat::OpenAI => {
                        format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
                    }
                    ApiFormat::QwenVision => {
                        format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
                    }
                    ApiFormat::Qwen => {
                        // Qwen 原生 API 路径
                        format!("{}/services/aigc/text-generation/generation", self.endpoint.trim_end_matches('/'))
                    }
                }
            }
        };

        // 构建请求，所有 header 从配置读取
        let mut request_builder = self.client
            .post(&full_endpoint)
            .header("Authorization", auth_value)
            .header("Content-Type", "application/json");
        
        // BigModel API 特殊处理（从 endpoint 判断，可配置化）
        if self.endpoint.contains("bigmodel.cn") {
            request_builder = request_builder
                .header("Accept", "application/json")
                .header("User-Agent", "agentflow/1.0.0");
        }

        let response = request_builder
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("HTTP request error: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Failed to read response: {}", e)))?;
        
        if !status.is_success() {
            // 优化错误信息，避免打印完整的 base64 数据
            let body_str = serde_json::to_string(&body).unwrap_or_default();
            // 如果请求体包含 base64，截断显示
            let truncated_body = if body_str.len() > 500 {
                // 检查是否包含 base64 数据
                if body_str.contains("base64") || body_str.len() > 1000 {
                    format!("{}...(请求体过长，已截断，总长度: {} 字节)", &body_str[..500], body_str.len())
                } else {
                    format!("{}...(已截断，总长度: {} 字节)", &body_str[..500], body_str.len())
                }
            } else {
                body_str
            };
            
            // 截断响应文本
            let truncated_response = if response_text.len() > 500 {
                format!("{}...(响应过长，已截断，总长度: {} 字节)", &response_text[..500], response_text.len())
            } else {
                response_text
            };
            
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "Request failed with status {}: {}\nEndpoint: {}\nRequest body (truncated): {}",
                status,
                truncated_response,
                full_endpoint,
                truncated_body
            )));
        }
        
        let payload: Value = serde_json::from_str(&response_text)
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!(
                "Response parse error: {}\nResponse body: {}",
                e,
                if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text.clone()
                }
            )))?;

        let content = match &self.format {
            ApiFormat::OpenAI => {
                payload["choices"][0]["message"]["content"]
                    .as_str()
            }
            ApiFormat::QwenVision => {
                payload["choices"][0]["message"]["content"]
                    .as_str()
                    .or_else(|| payload["output"]["text"].as_str())
            }
            ApiFormat::Qwen => {
                payload["output"]["text"]
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
            }
        }
        .ok_or_else(|| {
            AgentFlowError::Other(anyhow::anyhow!(
                "missing content in response: {}",
                serde_json::to_string(&payload).unwrap_or_default()
            ))
        })?;

        Ok(LlmResponse {
            content: content.to_string(),
            metadata: Some(payload),
        })
    }

    /// 实现真正的 SSE 流式响应
    /// 
    /// 注意：对于不支持流式的格式（如 Qwen），会降级到默认的逐字符流式输出
    fn complete_stream(&self, request: LlmRequest) -> LlmStream {
        // 使用默认的逐字符流式输出实现
        // TODO: 未来实现真正的 SSE 流式响应
        let request = Arc::new(request);
        let client = self.clone_dyn();
        
        Box::pin(futures::stream::unfold(
            (request, client, None::<String>, 0usize),
            move |(req, client, mut full_content, mut pos)| async move {
                if full_content.is_none() {
                    // 添加超时机制（5分钟超时）
                    use tokio::time::{timeout, Duration};
                    match timeout(Duration::from_secs(300), client.complete((*req).clone())).await {
                        Ok(Ok(response)) => {
                            full_content = Some(response.content);
                            pos = 0;
                        }
                        Ok(Err(e)) => {
                            return Some((Err(e), (req, client, full_content, pos)));
                        }
                        Err(_) => {
                            return Some((
                                Err(AgentFlowError::Other(anyhow::anyhow!(
                                    "LLM request timed out after 5 minutes"
                                ))),
                                (req, client, full_content, pos),
                            ));
                        }
                    }
                }
                
                let content = match full_content.as_ref() {
                    Some(c) => c,
                    None => return Some((Err(AgentFlowError::Other(anyhow::anyhow!("Stream response content is empty"))), (req, client, full_content, pos))),
                };
                if pos < content.len() {
                    let char_start = pos;
                    let ch = match content[char_start..].chars().next() {
                        Some(c) => c,
                        None => return Some((Err(AgentFlowError::Other(anyhow::anyhow!("Character parsing failed"))), (req, client, full_content, pos))),
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

    fn clone_dyn(&self) -> DynLlmClient {
        Arc::new(GenericHttpClient {
            client: self.client.clone(),
            endpoint: self.endpoint.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            format: self.format.clone(),
            auth_header: self.auth_header.clone(),
        })
    }
}

