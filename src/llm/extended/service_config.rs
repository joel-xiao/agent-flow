use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;

use super::json_config::{JsonApiConfig, JsonApiClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub services: Vec<ServiceDefinition>,
    pub api_graph: ApiGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub name: String,
    pub r#type: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub default_headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGraph {
    pub name: String,
    pub nodes: Vec<ApiNode>,
    #[serde(default)]
    pub edges: Vec<ApiEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiNode {
    pub id: String,
    pub r#type: String,
    pub service: String,
    pub path: String,
    pub method: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<Value>,
}

impl ServiceConfig {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse service config: {}", e)))
    }

    pub fn from_value(value: Value) -> Result<Self> {
        serde_json::from_value(value)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse service config value: {}", e)))
    }

    pub fn get_service(&self, name: &str) -> Option<&ServiceDefinition> {
        self.services.iter().find(|s| s.name == name)
    }

    pub fn create_client(&self, service_name: &str) -> Result<JsonApiClient> {
        let service = self.get_service(service_name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Service '{}' not found", service_name)))?;

        let mut json_config = JsonApiConfig {
            base_url: service.base_url.clone(),
            api_key: service.api_key.clone(),
            auth_header: service.auth_header.clone(),
            default_headers: service.default_headers.clone().unwrap_or_default(),
            endpoints: HashMap::new(),
        };

        for node in &self.api_graph.nodes {
            if node.service == service_name {
                json_config.endpoints.insert(node.id.clone(), super::json_config::EndpointConfig {
                    path: node.path.clone(),
                    method: Some(node.method.clone()),
                    default_headers: None,
                    response_transform: None,
                });
            }
        }

        Ok(JsonApiClient::from_config(json_config))
    }

    pub fn list_services(&self) -> Vec<&str> {
        self.services.iter().map(|s| s.name.as_str()).collect()
    }

    pub fn get_node(&self, node_id: &str) -> Option<&ApiNode> {
        self.api_graph.nodes.iter().find(|n| n.id == node_id)
    }

    pub fn list_nodes_by_service(&self, service_name: &str) -> Vec<&ApiNode> {
        self.api_graph.nodes.iter()
            .filter(|n| n.service == service_name)
            .collect()
    }

    pub fn list_all_nodes(&self) -> &[ApiNode] {
        &self.api_graph.nodes
    }

    pub fn get_edges_from(&self, node_id: &str) -> Vec<&ApiEdge> {
        self.api_graph.edges.iter()
            .filter(|e| e.from == node_id)
            .collect()
    }

    pub fn get_edges_to(&self, node_id: &str) -> Vec<&ApiEdge> {
        self.api_graph.edges.iter()
            .filter(|e| e.to == node_id)
            .collect()
    }
}

pub struct ServiceManager {
    config: ServiceConfig,
    clients: HashMap<String, JsonApiClient>,
}

impl ServiceManager {
    pub fn from_json(json: &str) -> Result<Self> {
        let config = ServiceConfig::from_json(json)?;
        let mut clients = HashMap::new();

        for service in &config.services {
            if let Ok(client) = config.create_client(&service.name) {
                clients.insert(service.name.clone(), client);
            }
        }

        Ok(Self { config, clients })
    }

    pub fn from_config(config: ServiceConfig) -> Self {
        let mut clients = HashMap::new();
        for service in &config.services {
            if let Ok(client) = config.create_client(&service.name) {
                clients.insert(service.name.clone(), client);
            }
        }
        Self { config, clients }
    }

    pub fn get_client(&self, service_name: &str) -> Option<&JsonApiClient> {
        self.clients.get(service_name)
    }

    pub fn get_config(&self) -> &ServiceConfig {
        &self.config
    }

    pub async fn call_node(
        &self,
        node_id: &str,
        body: Option<Value>,
        path_params: Option<HashMap<String, String>>,
        query_params: Option<HashMap<String, String>>,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value> {
        let node = self.config.get_node(node_id)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Node '{}' not found", node_id)))?;

        let client = self.get_client(&node.service)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Client for service '{}' not found", node.service)))?;

        let request = super::json_config::ApiCallRequest {
            endpoint: node_id.to_string(),
            method: Some(node.method.clone()),
            path_params,
            query_params,
            body,
            headers,
        };

        client.call(&request).await
    }
}

