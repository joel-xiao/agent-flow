// 此文件已重构，配置结构已移动到子模块
// 保留此文件以保持向后兼容，重新导出所有公共接口

// 这些模块在 src/config/mod.rs 中已经声明，这里重新导出
pub use super::graph::{GraphConfig, GraphNode, GraphEdge};
pub use super::conditions::Condition;
pub use super::nodes::{
    ServiceConfig, AgentNodeConfig, DecisionNodeConfig, DecisionBranchConfig,
    JoinNodeConfig, LoopNodeConfig, WorkflowConfig,
};
pub use super::agent_config::AgentConfig;
pub use super::agent_rules::{
    AgentRules, FieldExtractionRules, PromptBuildingRules,
    RoutingRules, PayloadBuildingRules, ImageProcessingRules,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_graph_config_serialization() {
        let config = GraphConfig {
            name: "test_config".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node1".to_string(),
                    node_type: "agent_node".to_string(),
                    config: json!({"agent": "agent1"}),
                    workflow: Some("workflow1".to_string()),
                    metadata: None,
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "node1".to_string(),
                    to: "node2".to_string(),
                    edge_type: "always".to_string(),
                    name: None,
                    condition: None,
                    workflow: Some("workflow1".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: GraphConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.name, "test_config");
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.edges.len(), 1);
    }
}
