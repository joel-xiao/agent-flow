use crate::error::{AgentFlowError, Result};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// LangGraph 风格的统一图配置
///
/// 所有节点和边都在统一的数组中,完全由 JSON 驱动
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    pub name: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// 统一的图节点定义
///
/// 所有节点都使用相同的结构,通过 `type` 字段区分类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// 节点唯一标识
    pub id: String,

    /// 节点类型: service, agent, agent_node, decision_node, join_node,
    /// loop_node, terminal_node, workflow, tool_node
    #[serde(rename = "type")]
    pub node_type: String,

    /// 节点配置,根据类型不同而不同
    pub config: Value,

    /// 可选的 workflow 标识,用于将节点组织到工作流中
    #[serde(default)]
    pub workflow: Option<String>,

    /// 可选的元数据
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// 统一的图边定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// 源节点 ID
    pub from: String,

    /// 目标节点 ID
    pub to: String,

    /// 边类型: always, conditional
    #[serde(rename = "type", default = "default_edge_type")]
    pub edge_type: String,

    /// 可选的边名称
    #[serde(default)]
    pub name: Option<String>,

    /// 条件配置(仅用于 conditional 类型)
    #[serde(default)]
    pub condition: Option<crate::config::conditions::Condition>,

    /// 可选的 workflow 标识
    #[serde(default)]
    pub workflow: Option<String>,
}

fn default_edge_type() -> String {
    "always".to_string()
}

impl GraphConfig {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse graph config: {}", e)))
    }

    pub fn from_value(value: Value) -> Result<Self> {
        serde_json::from_value(value).map_err(|e| {
            AgentFlowError::Other(anyhow!("Failed to parse graph config value: {}", e))
        })
    }

    /// 根据 ID 获取节点
    pub fn get_node(&self, node_id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// 根据类型获取节点
    pub fn get_nodes_by_type(&self, node_type: &str) -> Vec<&GraphNode> {
        self.nodes
            .iter()
            .filter(|n| n.node_type == node_type)
            .collect()
    }

    /// 根据 workflow 获取节点
    pub fn get_nodes_by_workflow(&self, workflow_id: &str) -> Vec<&GraphNode> {
        self.nodes
            .iter()
            .filter(|n| {
                n.workflow
                    .as_ref()
                    .map(|w| w == workflow_id)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// 获取所有 agent 节点
    pub fn get_agents(&self) -> Vec<&GraphNode> {
        self.get_nodes_by_type("agent")
    }

    /// 获取所有 workflow 节点
    pub fn get_workflows(&self) -> Vec<&GraphNode> {
        self.get_nodes_by_type("workflow")
    }

    /// 获取从指定节点出发的边
    pub fn get_edges_from(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// 获取指向指定节点的边
    pub fn get_edges_to(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    /// 获取指定 workflow 的所有边
    pub fn get_edges_by_workflow(&self, workflow_id: &str) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| {
                e.workflow
                    .as_ref()
                    .map(|w| w == workflow_id)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// 验证配置的完整性
    pub fn validate(&self) -> Result<()> {
        let mut node_ids = std::collections::HashSet::new();
        for node in &self.nodes {
            if !node_ids.insert(&node.id) {
                return Err(AgentFlowError::Other(anyhow!(
                    "Duplicate node ID: {}",
                    node.id
                )));
            }
        }

        for edge in &self.edges {
            if !node_ids.contains(&edge.from) {
                return Err(AgentFlowError::Other(anyhow!(
                    "Edge references unknown node: {}",
                    edge.from
                )));
            }
            if !node_ids.contains(&edge.to) {
                return Err(AgentFlowError::Other(anyhow!(
                    "Edge references unknown node: {}",
                    edge.to
                )));
            }
        }

        for workflow_node in self.get_workflows() {
            let _workflow_id = &workflow_node.id;
        }

        Ok(())
    }
}
