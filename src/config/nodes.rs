use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::graph::GraphNode;
use super::conditions::Condition;

/// Service 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub service_type: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default)]
    pub default_headers: Option<std::collections::HashMap<String, String>>,
}

/// Agent Node 配置(工作流中的 agent 节点)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNodeConfig {
    pub agent: String,  // 引用 agent 节点的 ID
}

/// Decision Node 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionNodeConfig {
    pub policy: String,  // first_match, all_matches
    pub branches: Vec<DecisionBranchConfig>,
}

/// Decision Branch 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionBranchConfig {
    pub target: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<Condition>,
}

/// Join Node 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinNodeConfig {
    pub strategy: String,  // all, any, count:N
    pub inbound: Vec<String>,
}

/// Loop Node 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNodeConfig {
    pub entry: String,
    #[serde(default)]
    pub condition: Option<Condition>,
    #[serde(default)]
    pub max_iterations: Option<u32>,
    #[serde(default)]
    pub exit: Option<String>,
}

/// Workflow 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub start: String,
    #[serde(default)]
    pub parameters: Vec<Value>,
    #[serde(default)]
    pub variables: Vec<Value>,
}

impl GraphNode {
    /// 尝试解析为 Service 配置
    pub fn as_service(&self) -> Result<ServiceConfig> {
        if self.node_type != "service" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not a service", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse service config: {}", e)))
    }

    /// 尝试解析为 Workflow 配置
    pub fn as_workflow(&self) -> Result<WorkflowConfig> {
        if self.node_type != "workflow" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not a workflow", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse workflow config: {}", e)))
    }

    /// 尝试解析为 AgentNode 配置
    pub fn as_agent_node(&self) -> Result<AgentNodeConfig> {
        if self.node_type != "agent_node" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not an agent_node", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse agent_node config: {}", e)))
    }

    /// 尝试解析为 DecisionNode 配置
    pub fn as_decision_node(&self) -> Result<DecisionNodeConfig> {
        if self.node_type != "decision_node" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not a decision_node", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse decision_node config: {}", e)))
    }

    /// 尝试解析为 JoinNode 配置
    pub fn as_join_node(&self) -> Result<JoinNodeConfig> {
        if self.node_type != "join_node" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not a join_node", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse join_node config: {}", e)))
    }

    /// 尝试解析为 LoopNode 配置
    pub fn as_loop_node(&self) -> Result<LoopNodeConfig> {
        if self.node_type != "loop_node" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not a loop_node", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse loop_node config: {}", e)))
    }
}

