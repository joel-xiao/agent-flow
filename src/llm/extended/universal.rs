use std::collections::HashMap;
use std::sync::Arc;
use anyhow::anyhow;
use reqwest::{Client, Method};
use serde_json::Value;
use crate::error::{Result, AgentFlowError};

use crate::llm::config::ApiEndpointConfig;

pub struct ApiCallConfig {
    pub method: Method,
    pub endpoint_key: String,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<Value>,
    pub multipart: Option<reqwest::multipart::Form>,
    pub headers: HashMap<String, String>,
    pub response_transform: Option<fn(Value) -> Result<Value>>,
}

impl ApiCallConfig {
    pub fn new(method: Method, endpoint_key: impl Into<String>) -> Self {
        Self {
            method,
            endpoint_key: endpoint_key.into(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            multipart: None,
            headers: HashMap::new(),
            response_transform: None,
        }
    }

    pub fn with_path_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.path_params.insert(key.into(), value.into());
        self
    }

    pub fn with_query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }

    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_multipart(mut self, form: reqwest::multipart::Form) -> Self {
        self.multipart = Some(form);
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_response_transform(mut self, transform: fn(Value) -> Result<Value>) -> Self {
        self.response_transform = Some(transform);
        self
    }
}

pub struct UniversalApiClient {
    client: Client,
    config: ApiEndpointConfig,
    api_key: String,
    default_headers: HashMap<String, String>,
}

impl UniversalApiClient {
    pub fn new(config: ApiEndpointConfig, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            config,
            api_key: api_key.into(),
            default_headers: HashMap::new(),
        }
    }

    pub fn with_default_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }

    pub async fn call(&self, call_config: ApiCallConfig) -> Result<Value> {
        let mut url = self.config.get_endpoint(&call_config.endpoint_key)
            .ok_or_else(|| AgentFlowError::Other(anyhow!(
                "Endpoint '{}' not configured", call_config.endpoint_key
            )))?;

        for (key, value) in &call_config.path_params {
            url = url.replace(&format!("{{{}}}", key), value);
        }

        if !call_config.query_params.is_empty() {
            let query_string = call_config.query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, query_string);
        }

        let auth_value = self.config.auth_header.as_ref()
            .map(|h| format!("{} {}", h, self.api_key))
            .unwrap_or_else(|| format!("Bearer {}", self.api_key));

        let mut request_builder = self.client
            .request(call_config.method.clone(), &url)
            .header("Authorization", auth_value);

        for (key, value) in &self.config.default_headers {
            request_builder = request_builder.header(key, value);
        }

        for (key, value) in &self.default_headers {
            request_builder = request_builder.header(key, value);
        }

        for (key, value) in &call_config.headers {
            request_builder = request_builder.header(key, value);
        }

        if let Some(multipart) = call_config.multipart {
            request_builder = request_builder.multipart(multipart);
        } else if let Some(body) = call_config.body {
            request_builder = request_builder
                .header("Content-Type", "application/json")
                .json(&body);
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow!(
                "HTTP request error: {}", e
            )))?;

        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| AgentFlowError::Other(anyhow!(
                "Failed to read response: {}", e
            )))?;

        if !status.is_success() {
            return Err(AgentFlowError::Other(anyhow!(
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

        let mut payload: Value = serde_json::from_str(&response_text)
            .map_err(|e| AgentFlowError::Other(anyhow!(
                "Response parse error: {}\nResponse body: {}",
                e,
                if response_text.len() > 500 {
                    format!("{}...", &response_text[..500])
                } else {
                    response_text
                }
            )))?;

        if let Some(transform) = call_config.response_transform {
            payload = transform(payload)?;
        }

        Ok(payload)
    }

    pub async fn call_raw(&self, call_config: ApiCallConfig) -> Result<Vec<u8>> {
        let mut url = self.config.get_endpoint(&call_config.endpoint_key)
            .ok_or_else(|| AgentFlowError::Other(anyhow!(
                "Endpoint '{}' not configured", call_config.endpoint_key
            )))?;

        for (key, value) in &call_config.path_params {
            url = url.replace(&format!("{{{}}}", key), value);
        }

        let auth_value = self.config.auth_header.as_ref()
            .map(|h| format!("{} {}", h, self.api_key))
            .unwrap_or_else(|| format!("Bearer {}", self.api_key));

        let mut request_builder = self.client
            .request(call_config.method.clone(), &url)
            .header("Authorization", auth_value);

        for (key, value) in &self.config.default_headers {
            request_builder = request_builder.header(key, value);
        }

        for (key, value) in &self.default_headers {
            request_builder = request_builder.header(key, value);
        }

        for (key, value) in &call_config.headers {
            request_builder = request_builder.header(key, value);
        }

        if let Some(multipart) = call_config.multipart {
            request_builder = request_builder.multipart(multipart);
        } else if let Some(body) = call_config.body {
            request_builder = request_builder
                .header("Content-Type", "application/json")
                .json(&body);
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow!(
                "HTTP request error: {}", e
            )))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AgentFlowError::Other(anyhow!(
                "Request failed with status {}: {}", status, error_text
            )));
        }

        let bytes = response.bytes().await
            .map_err(|e| AgentFlowError::Other(anyhow!(
                "Failed to read response: {}", e
            )))?;

        Ok(bytes.to_vec())
    }
}

pub struct ApiBuilder {
    client: Arc<UniversalApiClient>,
}

impl ApiBuilder {
    pub fn new(client: UniversalApiClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    pub fn get(&self, endpoint_key: impl Into<String>) -> ApiCallBuilder {
        ApiCallBuilder::new(self.client.clone(), Method::GET, endpoint_key.into())
    }

    pub fn post(&self, endpoint_key: impl Into<String>) -> ApiCallBuilder {
        ApiCallBuilder::new(self.client.clone(), Method::POST, endpoint_key.into())
    }

    pub fn put(&self, endpoint_key: impl Into<String>) -> ApiCallBuilder {
        ApiCallBuilder::new(self.client.clone(), Method::PUT, endpoint_key.into())
    }

    pub fn delete(&self, endpoint_key: impl Into<String>) -> ApiCallBuilder {
        ApiCallBuilder::new(self.client.clone(), Method::DELETE, endpoint_key.into())
    }
}

pub struct ApiCallBuilder {
    client: Arc<UniversalApiClient>,
    config: ApiCallConfig,
}

impl ApiCallBuilder {
    fn new(client: Arc<UniversalApiClient>, method: Method, endpoint_key: String) -> Self {
        Self {
            client,
            config: ApiCallConfig::new(method, endpoint_key),
        }
    }

    pub fn path_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.path_params.insert(key.into(), value.into());
        self
    }

    pub fn query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.query_params.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: Value) -> Self {
        self.config.body = Some(body);
        self
    }

    pub fn multipart(mut self, form: reqwest::multipart::Form) -> Self {
        self.config.multipart = Some(form);
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.headers.insert(key.into(), value.into());
        self
    }

    pub fn response_transform(mut self, transform: fn(Value) -> Result<Value>) -> Self {
        self.config.response_transform = Some(transform);
        self
    }

    pub async fn call(self) -> Result<Value> {
        self.client.call(self.config).await
    }

    pub async fn call_raw(self) -> Result<Vec<u8>> {
        self.client.call_raw(self.config).await
    }
}

