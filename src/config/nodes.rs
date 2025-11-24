use super::conditions::Condition;
use super::graph::GraphNode;
use crate::error::{AgentFlowError, Result};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Decision Node 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionNodeConfig {
    pub policy: String, // first_match, all_matches
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
    pub strategy: String, // all, any, count:N
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

/// Tool Node 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNodeConfig {
    pub pipeline: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl GraphNode {
    /// 尝试解析为 Workflow 配置
    pub fn as_workflow(&self) -> Result<WorkflowConfig> {
        if self.node_type != "workflow" {
            return Err(AgentFlowError::Other(anyhow!(
                "Node {} is not a workflow",
                self.id
            )));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse workflow config: {}", e)))
    }

    /// 尝试解析为 DecisionNode 配置
    pub fn as_decision_node(&self) -> Result<DecisionNodeConfig> {
        if self.node_type != "decision_node" {
            return Err(AgentFlowError::Other(anyhow!(
                "Node {} is not a decision_node",
                self.id
            )));
        }
        serde_json::from_value(self.config.clone()).map_err(|e| {
            AgentFlowError::Other(anyhow!("Failed to parse decision_node config: {}", e))
        })
    }

    /// 尝试解析为 JoinNode 配置
    pub fn as_join_node(&self) -> Result<JoinNodeConfig> {
        if self.node_type != "join_node" {
            return Err(AgentFlowError::Other(anyhow!(
                "Node {} is not a join_node",
                self.id
            )));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse join_node config: {}", e)))
    }

    /// 尝试解析为 LoopNode 配置
    pub fn as_loop_node(&self) -> Result<LoopNodeConfig> {
        if self.node_type != "loop_node" {
            return Err(AgentFlowError::Other(anyhow!(
                "Node {} is not a loop_node",
                self.id
            )));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse loop_node config: {}", e)))
    }

    /// 尝试解析为 ToolNode 配置
    pub fn as_tool_node(&self) -> Result<ToolNodeConfig> {
        if self.node_type != "tool_node" {
            return Err(AgentFlowError::Other(anyhow!(
                "Node {} is not a tool_node",
                self.id
            )));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse tool_node config: {}", e)))
    }
}
