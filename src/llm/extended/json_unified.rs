use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;

use super::json_config::{JsonApiConfig, JsonApiClient, ApiCallRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedJsonConfig {
    #[serde(default)]
    pub api_configs: Vec<ApiProviderConfig>,
    #[serde(default)]
    pub api_calls: HashMap<String, HashMap<String, ApiCallConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProviderConfig {
    pub name: String,
    pub driver: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub default_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub endpoints: Option<HashMap<String, EndpointDefinition>>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDefinition {
    pub path: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub default_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallConfig {
    #[serde(default)]
    pub name: String,
    pub provider: String,
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
    #[serde(default)]
    pub description: Option<String>,
}

impl UnifiedJsonConfig {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse unified JSON config: {}", e)))
    }

    pub fn from_value(value: Value) -> Result<Self> {
        serde_json::from_value(value)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse config value: {}", e)))
    }

    pub fn get_provider(&self, name: &str) -> Option<&ApiProviderConfig> {
        self.api_configs.iter().find(|c| c.name == name)
    }

    pub fn get_api_call(&self, category: &str, name: &str) -> Option<&ApiCallConfig> {
        self.api_calls.get(category)
            .and_then(|category_calls| category_calls.get(name))
    }

    pub fn list_all_api_calls(&self) -> Vec<(String, String, ApiCallConfig)> {
        let mut result = Vec::new();
        for (category, calls) in &self.api_calls {
            for (name, config) in calls {
                result.push((category.clone(), name.clone(), config.clone()));
            }
        }
        result
    }

    pub fn create_client(&self, provider_name: &str) -> Result<JsonApiClient> {
        let provider = self.get_provider(provider_name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Provider '{}' not found", provider_name)))?;

        let base_url = provider.base_url.as_ref()
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Provider '{}' missing base_url", provider_name)))?;

        let api_key = provider.api_key.as_ref()
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Provider '{}' missing api_key", provider_name)))?;

        let mut json_config = JsonApiConfig {
            base_url: base_url.clone(),
            api_key: api_key.clone(),
            auth_header: provider.auth_header.clone(),
            default_headers: provider.default_headers.clone().unwrap_or_default(),
            endpoints: HashMap::new(),
        };

        if let Some(ref endpoints) = provider.endpoints {
            for (name, endpoint) in endpoints {
                json_config.endpoints.insert(name.clone(), super::json_config::EndpointConfig {
                    path: endpoint.path.clone(),
                    method: endpoint.method.clone(),
                    default_headers: endpoint.default_headers.clone(),
                    response_transform: None,
                });
            }
        }

        Ok(JsonApiClient::from_config(json_config))
    }

    pub fn execute_api_call(&self, category: &str, call_name: &str) -> Result<ApiCallRequest> {
        let api_call = self.get_api_call(category, call_name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("API call '{}/{}' not found", category, call_name)))?;

        let provider = self.get_provider(&api_call.provider)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Provider '{}' not found", api_call.provider)))?;

        let endpoint_name = &api_call.endpoint;
        if let Some(ref endpoints) = provider.endpoints {
            if !endpoints.contains_key(endpoint_name) {
                return Err(AgentFlowError::Other(anyhow!(
                    "Endpoint '{}' not found in provider '{}'", endpoint_name, api_call.provider
                )));
            }
        }

        Ok(ApiCallRequest {
            endpoint: api_call.endpoint.clone(),
            method: api_call.method.clone(),
            path_params: api_call.path_params.clone(),
            query_params: api_call.query_params.clone(),
            body: api_call.body.clone(),
            headers: api_call.headers.clone(),
        })
    }
}

pub struct UnifiedApiManager {
    config: UnifiedJsonConfig,
    clients: HashMap<String, JsonApiClient>,
}

impl UnifiedApiManager {
    pub fn from_json(json: &str) -> Result<Self> {
        let config = UnifiedJsonConfig::from_json(json)?;
        let mut clients = HashMap::new();

        for provider in &config.api_configs {
            if let Ok(client) = config.create_client(&provider.name) {
                clients.insert(provider.name.clone(), client);
            }
        }

        Ok(Self { config, clients })
    }

    pub fn from_config(config: UnifiedJsonConfig) -> Self {
        let mut clients = HashMap::new();
        for provider in &config.api_configs {
            if let Ok(client) = config.create_client(&provider.name) {
                clients.insert(provider.name.clone(), client);
            }
        }
        Self { config, clients }
    }

    pub fn get_client(&self, provider_name: &str) -> Option<&JsonApiClient> {
        self.clients.get(provider_name)
    }

    pub async fn call(&self, category: &str, call_name: &str) -> Result<Value> {
        let api_call = self.config.execute_api_call(category, call_name)?;
        let provider = self.config.get_api_call(category, call_name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("API call '{}/{}' not found", category, call_name)))?;
        
        let client = self.get_client(&provider.provider)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Client for provider '{}' not found", provider.provider)))?;

        client.call(&api_call).await
    }

    pub async fn call_by_full_name(&self, full_name: &str) -> Result<Value> {
        let parts: Vec<&str> = full_name.splitn(2, '/').collect();
        if parts.len() == 2 {
            self.call(parts[0], parts[1]).await
        } else {
            Err(AgentFlowError::Other(anyhow!("Invalid API call name format: {}, expected 'category/name'", full_name)))
        }
    }

    pub fn get_config(&self) -> &UnifiedJsonConfig {
        &self.config
    }
}

