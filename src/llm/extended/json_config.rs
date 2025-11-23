use std::collections::HashMap;
use std::sync::Arc;
use anyhow::anyhow;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Result, AgentFlowError};

use crate::llm::config::ApiEndpointConfig;
use super::universal::{UniversalApiClient, ApiBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiConfig {
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub default_headers: HashMap<String, String>,
    pub endpoints: HashMap<String, EndpointConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub path: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub default_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub response_transform: Option<String>,
}

impl JsonApiConfig {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse JSON config: {}", e)))
    }

    pub fn from_value(value: Value) -> Result<Self> {
        serde_json::from_value(value)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse config value: {}", e)))
    }

    pub fn to_api_endpoint_config(&self) -> ApiEndpointConfig {
        let mut config = ApiEndpointConfig::new(&self.base_url);

        if let Some(ref auth_header) = self.auth_header {
            config = config.with_auth_header(auth_header);
        }

        for (key, value) in &self.default_headers {
            config = config.with_default_header(key, value);
        }

        for (name, endpoint) in &self.endpoints {
            config = config.with_endpoint(name, &endpoint.path);
        }

        config
    }

    pub fn create_client(&self) -> ApiBuilder {
        let config = self.to_api_endpoint_config();
        let client = UniversalApiClient::new(config, &self.api_key);
        ApiBuilder::new(client)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallRequest {
    pub endpoint: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path_params: Option<HashMap<String, String>>,
    #[serde(default)]
    pub query_params: Option<HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<Value>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

impl ApiCallRequest {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse API call request: {}", e)))
    }

    pub fn from_value(value: Value) -> Result<Self> {
        serde_json::from_value(value)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse request value: {}", e)))
    }

    pub async fn execute(&self, api: &ApiBuilder) -> Result<Value> {
        let method = self.method.as_deref().unwrap_or("POST");
        let http_method = match method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => return Err(AgentFlowError::Other(anyhow!("Unsupported HTTP method: {}", method))),
        };

        let mut builder = match http_method {
            Method::GET => api.get(&self.endpoint),
            Method::POST => api.post(&self.endpoint),
            Method::PUT => api.put(&self.endpoint),
            Method::DELETE => api.delete(&self.endpoint),
            Method::PATCH => api.post(&self.endpoint),
            _ => unreachable!(),
        };

        if let Some(ref path_params) = self.path_params {
            for (key, value) in path_params {
                builder = builder.path_param(key, value);
            }
        }

        if let Some(ref query_params) = self.query_params {
            for (key, value) in query_params {
                builder = builder.query_param(key, value);
            }
        }

        if let Some(ref body) = self.body {
            builder = builder.body(body.clone());
        }

        if let Some(ref headers) = self.headers {
            for (key, value) in headers {
                builder = builder.header(key, value);
            }
        }

        builder.call().await
    }
}

pub struct JsonApiClient {
    api: ApiBuilder,
    config: JsonApiConfig,
}

impl JsonApiClient {
    pub fn from_json_config(config_json: &str) -> Result<Self> {
        let config = JsonApiConfig::from_json(config_json)?;
        let api = config.create_client();
        Ok(Self { api, config })
    }

    pub fn from_config(config: JsonApiConfig) -> Self {
        let api = config.create_client();
        Self { api, config }
    }

    pub async fn call(&self, request: &ApiCallRequest) -> Result<Value> {
        request.execute(&self.api).await
    }

    pub async fn call_from_json(&self, request_json: &str) -> Result<Value> {
        let request = ApiCallRequest::from_json(request_json)?;
        self.call(&request).await
    }

    pub fn get_api(&self) -> &ApiBuilder {
        &self.api
    }

    pub fn get_config(&self) -> &JsonApiConfig {
        &self.config
    }
}

