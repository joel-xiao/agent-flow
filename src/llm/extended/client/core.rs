use reqwest::Client;
use serde_json::Value;
use crate::error::{Result, AgentFlowError};
use crate::llm::config::ApiEndpointConfig;

/// GenericApiClient 核心结构
pub struct GenericApiClient {
    pub client: Client,
    pub config: ApiEndpointConfig,
    pub api_key: String,
}

impl GenericApiClient {
    pub fn new(config: ApiEndpointConfig, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            config,
            api_key: api_key.into(),
        }
    }

    /// 执行 HTTP 请求的通用方法
    pub async fn request(
        &self,
        method: &str,
        endpoint_name: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = self.config.get_endpoint(endpoint_name)
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!(
                "Endpoint '{}' not configured", endpoint_name
            )))?;

        let auth_value = self.config.auth_header.as_ref()
            .map(|h| format!("{} {}", h, self.api_key))
            .unwrap_or_else(|| format!("Bearer {}", self.api_key));

        let mut request_builder = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(AgentFlowError::Other(anyhow::anyhow!(
                "Unsupported HTTP method: {}", method
            ))),
        };

        request_builder = request_builder
            .header("Authorization", auth_value)
            .header("Content-Type", "application/json");

        for (key, value) in &self.config.default_headers {
            request_builder = request_builder.header(key, value);
        }

        if let Some(body) = body {
            request_builder = request_builder.json(&body);
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!(
                "HTTP request error: {}", e
            )))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!(
                "Failed to read response: {}", e
            )))?;

        if !status.is_success() {
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "Request failed with status {}: {}\nEndpoint: {}",
                status,
                if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text
                },
                url
            )));
        }

        serde_json::from_str(&response_text)
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!(
                "Response parse error: {}\nResponse body: {}",
                e,
                if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text
                }
            )))
    }

    /// 获取配置的基础 URL
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
}

