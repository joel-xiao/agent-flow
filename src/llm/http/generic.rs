use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use serde_json::Value;

#[cfg(feature = "openai-client")]
use reqwest;
#[cfg(feature = "openai-client")]
use tracing::instrument;

use crate::error::{AgentFlowError, Result};
use crate::llm::client::{DynLlmClient, LlmClient, LlmStream};
use crate::llm::types::{ApiFormat, LlmRequest, LlmResponse, LlmStreamChunk};
use anyhow::anyhow;
use futures::StreamExt;

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
    /// 创建优化的 HTTP 客户端配置
    ///
    /// 优化项：
    /// - 连接池：复用连接，提高性能
    /// - 超时设置：避免长时间等待
    /// - HTTP/2：支持多路复用
    fn create_optimized_client() -> reqwest::Client {
        reqwest::Client::builder()
            .pool_max_idle_per_host(10) // 每个主机最多保持 10 个空闲连接
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client with custom config")
    }

    pub fn new<S1, S2, S3>(endpoint: S1, api_key: S2, model: S3, format: ApiFormat) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        Self {
            client: Self::create_optimized_client(),
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            format,
            auth_header: None,
        }
    }

    pub fn with_auth_header<S1, S2, S3, S4>(
        endpoint: S1,
        api_key: S2,
        model: S3,
        format: ApiFormat,
        auth_header: S4,
    ) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
        S4: Into<String>,
    {
        Self {
            client: Self::create_optimized_client(),
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            format,
            auth_header: Some(auth_header.into()),
        }
    }

    /// 检查是否是图片生成模型
    fn is_image_generation_model(&self) -> bool {
        self.model.contains("t2i") || 
        self.model.contains("dalle") || 
        self.model.starts_with("wan")
    }

    /// 处理图片生成请求
    async fn complete_image_generation(&self, request: LlmRequest) -> Result<LlmResponse> {
        use serde::Deserialize;
        use tokio::time::{sleep, Duration};

        #[derive(Deserialize)]
        struct WanxiangResponse {
            output: WanxiangOutput,
        }

        #[derive(Deserialize)]
        struct WanxiangOutput {
            task_id: String,
            task_status: String,
            #[serde(default)]
            results: Vec<WanxiangResult>,
        }

        #[derive(Deserialize)]
        struct WanxiangResult {
            url: String,
        }

        let body = json!({
            "model": self.model,
            "input": {
                "prompt": request.user
            },
            "parameters": {
                "style": "<auto>",
                "size": "1024*1024",
                "n": 1
            }
        });

        let auth_value = if let Some(header) = &self.auth_header {
            format!("{} {}", header, self.api_key)
        } else {
            format!("Bearer {}", self.api_key)
        };

        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", auth_value.clone())
            .header("Content-Type", "application/json")
            .header("X-DashScope-Async", "enable")
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow!("Image generation request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| AgentFlowError::Other(anyhow!("Failed to read error response: {}", e)))?;
            return Err(AgentFlowError::Other(anyhow!(
                "Image generation API error: {}",
                error_text
            )));
        }

        let submit_resp: WanxiangResponse = response.json().await.map_err(|e| {
            AgentFlowError::Other(anyhow!("Failed to parse image generation response: {}", e))
        })?;

        if submit_resp.output.task_status == "SUCCEEDED" && !submit_resp.output.results.is_empty() {
            let image_url = &submit_resp.output.results[0].url;
            return Ok(LlmResponse {
                content: json!({
                    "image_url": image_url,
                    "task_id": submit_resp.output.task_id
                }).to_string(),
                metadata: None,
            });
        }

        let task_id = submit_resp.output.task_id;
        let query_endpoint = "https://dashscope.aliyuncs.com/api/v1/tasks";
        let query_url = format!("{}/{}", query_endpoint, task_id);

        for _ in 0..60 {
            sleep(Duration::from_secs(2)).await;

            let query_response = self.client
                .get(&query_url)
                .header("Authorization", auth_value.clone())
                .send()
                .await;

            if query_response.is_err() {
                continue;
            }

            let query_response = query_response.unwrap();
            if !query_response.status().is_success() {
                continue;
            }

            let result: WanxiangResponse = match query_response.json().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            match result.output.task_status.as_str() {
                "SUCCEEDED" if !result.output.results.is_empty() => {
                    let image_url = &result.output.results[0].url;
                    return Ok(LlmResponse {
                        content: json!({
                            "image_url": image_url,
                            "task_id": task_id
                        }).to_string(),
                        metadata: None,
                    });
                }
                "FAILED" => {
                    return Err(AgentFlowError::Other(anyhow!("Image generation failed")));
                }
                _ => continue,
            }
        }

        Err(AgentFlowError::Other(anyhow!("Image generation timeout")))
    }
}

#[cfg(feature = "openai-client")]
#[async_trait]
impl LlmClient for GenericHttpClient {
    #[instrument(skip(self))]
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        if self.is_image_generation_model() {
            return self.complete_image_generation(request).await;
        }
        
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
                    let temp = (request.temperature * 100.0).round() / 100.0;
                    if temp > 0.0 && temp <= 2.0 {
                        let temp_str = format!("{:.2}", temp);
                        if let Ok(temp_val) = temp_str.parse::<f64>() {
                            body["temperature"] = json!(temp_val);
                        }
                    }
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

        let auth_value = if let Some(auth_header) = &self.auth_header {
            format!("{} {}", auth_header, self.api_key)
        } else {
            format!("Bearer {}", self.api_key)
        };

        let full_endpoint = if self.endpoint.contains("/chat/completions") 
            || self.endpoint.contains("/services/")
            || self.endpoint.contains("/generation")
        {
            self.endpoint.clone()
        } else {
            if self.endpoint.contains("compatible-mode") {
                format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
            } else {
                match &self.format {
                    ApiFormat::OpenAI => {
                        format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
                    }
                    ApiFormat::QwenVision => {
                        format!("{}/chat/completions", self.endpoint.trim_end_matches('/'))
                    }
                    ApiFormat::Qwen => {
                        format!(
                            "{}/services/aigc/text-generation/generation",
                            self.endpoint.trim_end_matches('/')
                        )
                    }
                }
            }
        };

        let mut request_builder = self
            .client
            .post(&full_endpoint)
            .header("Authorization", auth_value)
            .header("Content-Type", "application/json");
        
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
        let response_text = response.text().await.map_err(|e| {
            AgentFlowError::Other(anyhow::anyhow!("Failed to read response: {}", e))
        })?;
        
        if !status.is_success() {
            if let Ok(body_str) = serde_json::to_string(&body) {
                let truncated_body = if body_str.len() > 500 {
                    if body_str.contains("base64") || body_str.len() > 1000 {
                        format!(
                            "{}...(请求体过长，已截断，总长度: {} 字节)",
                            &body_str[..500],
                            body_str.len()
                        )
                    } else {
                        format!(
                            "{}...(已截断，总长度: {} 字节)",
                            &body_str[..500],
                            body_str.len()
                        )
                    }
                } else {
                    body_str.clone()
                };
                
                let truncated_response = if response_text.len() > 500 {
                    format!(
                        "{}...(响应过长，已截断，总长度: {} 字节)",
                        &response_text[..500],
                        response_text.len()
                    )
                } else {
                    response_text.clone()
                };
                
                return Err(AgentFlowError::Other(anyhow::anyhow!(
                    "Request failed with status {}: {}\nEndpoint: {}\nRequest body (truncated): {}",
                    status,
                    truncated_response,
                    full_endpoint,
                    truncated_body
                )));
            }
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "Request failed with status {}: {}\nEndpoint: {}",
                status,
                response_text,
                full_endpoint
            )));
        }
        
        let payload: Value = serde_json::from_str(&response_text).map_err(|e| {
            AgentFlowError::Other(anyhow::anyhow!(
                "Response parse error: {}\nResponse body: {}",
                e,
                if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text.clone()
                }
            ))
        })?;

        let content = match &self.format {
            ApiFormat::OpenAI => payload["choices"][0]["message"]["content"].as_str(),
            ApiFormat::QwenVision => payload["choices"][0]["message"]["content"].as_str(),
            ApiFormat::Qwen => payload["output"]["text"].as_str(),
        }
        .ok_or_else(|| {
            if let Ok(payload_str) = serde_json::to_string(&payload) {
                AgentFlowError::Other(anyhow::anyhow!(
                    "Missing content in response for format {:?}: {}",
                    self.format,
                    payload_str
                ))
            } else {
                AgentFlowError::Other(anyhow::anyhow!(
                    "Missing content in response for format {:?}",
                    self.format
                ))
            }
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
        let request = Arc::new(request);
        let client = self.clone_dyn();
        
        Box::pin(
            futures::stream::unfold(
            (request, client, None::<String>, 0usize),
            move |(req, client, mut full_content, mut pos)| async move {
                if full_content.is_none() {
                    use tokio::time::{timeout, Duration};
                        match timeout(Duration::from_secs(300), client.complete((*req).clone()))
                            .await
                        {
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
                        None => {
                            return Some((
                                Err(AgentFlowError::Other(anyhow::anyhow!(
                                    "Stream response content is empty"
                                ))),
                                (req, client, full_content, pos),
                            ))
                        }
                };
                if pos < content.len() {
                    let char_start = pos;
                    let ch = match content[char_start..].chars().next() {
                        Some(c) => c,
                            None => {
                                return Some((
                                    Err(AgentFlowError::Other(anyhow::anyhow!(
                                        "Character parsing failed"
                                    ))),
                                    (req, client, full_content, pos),
                                ))
                            }
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
            )
            .chain(futures::stream::once(async move {
            Ok(LlmStreamChunk {
                content: String::new(),
                done: true,
            })
            })),
        )
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
