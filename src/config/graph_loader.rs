use super::conditions::Condition;
use super::graph::{GraphConfig, GraphNode};
use crate::error::{AgentFlowError, Result};
use crate::flow::loader::WorkflowBundle;
use anyhow::anyhow;
use serde_json::{json, Value};

/// 从 GraphConfig 加载工作流
impl GraphConfig {
    /// 加载指定的工作流
    pub fn load_workflow(&self, workflow_id: &str) -> Result<WorkflowBundle> {
        self.validate()?;

        let workflow_node = self.get_node(workflow_id).ok_or_else(|| {
            AgentFlowError::Other(anyhow!("Workflow '{}' not found", workflow_id))
        })?;

        let workflow_config = workflow_node.as_workflow()?;
        let workflow_name = workflow_config.name.clone();
        let start_node_id = workflow_config.start.clone();

        let workflow_nodes: Vec<&GraphNode> = self.get_nodes_by_workflow(workflow_id);

        let agents: Vec<Value> = workflow_nodes
            .iter()
            .filter(|node| node.node_type == "agent")
            .filter_map(|node| {
                node.as_agent().ok().map(|agent_config| {
                    let mut agent_json = json!({
                        "name": agent_config.name,
                        "driver": agent_config.driver,
                        "role": agent_config.role,
                        "prompt": agent_config.prompt,
                        "model": agent_config.model,
                        "intent": agent_config.intent,
                        "service": agent_config.service
                    });
                    
                    if let Some(endpoint) = &agent_config.endpoint {
                        agent_json["endpoint"] = json!(endpoint);
                    }
                    if let Some(api_key) = &agent_config.api_key {
                        agent_json["api_key"] = json!(api_key);
                    }
                    if let Some(metadata) = &agent_config.metadata {
                        agent_json["metadata"] = metadata.clone();
                    }
                    
                    if let Some(route_mode) = &agent_config.route_mode {
                        agent_json["route_mode"] = json!(route_mode);
                    }
                    if let Some(route_targets) = &agent_config.route_targets {
                        agent_json["route_targets"] = json!(route_targets);
                    }
                    if let Some(route_prompt) = &agent_config.route_prompt {
                        agent_json["route_prompt"] = json!(route_prompt);
                    }
                    if let Some(default_route) = &agent_config.default_route {
                        agent_json["default_route"] = json!(default_route);
                    }
                    
                    if let Some(rules) = &agent_config.rules {
                        if let Ok(rules_value) = serde_json::to_value(rules) {
                            agent_json["rules"] = rules_value;
                        }
                    }
                    
                    agent_json
                })
            })
            .collect();

        let mut nodes = Vec::new();
        for node in &workflow_nodes {
            match node.node_type.as_str() {
                "agent" => {
                    if let Ok(agent_config) = node.as_agent() {
                                nodes.push(json!({
                                    "kind": "agent",
                                    "name": node.id,
                                    "agent": agent_config.name
                                }));
                    }
                }
                "decision_node" => {
                    if let Ok(decision_config) = node.as_decision_node() {
                        let branches: Vec<Value> = decision_config
                            .branches
                            .iter()
                            .map(|branch| {
                                let mut branch_json = json!({
                                    "target": branch.target
                                });
                                if let Some(name) = &branch.name {
                                    branch_json["name"] = json!(name);
                                }
                                if let Some(condition) = &branch.condition {
                                    branch_json["condition"] = condition_to_json(condition);
                                }
                                branch_json
                            })
                            .collect();

                        nodes.push(json!({
                            "kind": "decision",
                            "name": node.id,
                            "policy": decision_config.policy,
                            "branches": branches
                        }));
                    }
                }
                "join_node" => {
                    if let Ok(join_config) = node.as_join_node() {
                        nodes.push(json!({
                            "kind": "join",
                            "name": node.id,
                            "strategy": join_config.strategy,
                            "inbound": join_config.inbound
                        }));
                    }
                }
                "loop_node" => {
                    if let Ok(loop_config) = node.as_loop_node() {
                        let mut loop_json = json!({
                            "kind": "loop",
                            "name": node.id,
                            "entry": loop_config.entry
                        });
                        if let Some(condition) = &loop_config.condition {
                            loop_json["condition"] = condition_to_json(condition);
                        }
                        if let Some(max_iterations) = loop_config.max_iterations {
                            loop_json["max_iterations"] = json!(max_iterations);
                        }
                        if let Some(exit) = &loop_config.exit {
                            loop_json["exit"] = json!(exit);
                        }
                        nodes.push(loop_json);
                    }
                }
                "terminal_node" => {
                    nodes.push(json!({
                        "kind": "terminal",
                        "name": node.id
                    }));
                }
                "tool_node" => {
                    if let Ok(tool_config) = node.as_tool_node() {
                        let mut tool_json = json!({
                            "kind": "tool",
                            "name": node.id,
                            "pipeline": tool_config.pipeline
                        });
                        if let Some(params) = &tool_config.params {
                            tool_json["params"] = params.clone();
                        }
                        nodes.push(tool_json);
                    }
                }
                _ => {
                }
            }
        }

        let workflow_edges = self.get_edges_by_workflow(workflow_id);
        let mut transitions = Vec::new();

        for edge in workflow_edges {
            let mut transition = json!({
                "from": edge.from,
                "to": edge.to
            });
            if let Some(name) = &edge.name {
                transition["name"] = json!(name);
            }
            if let Some(condition) = &edge.condition {
                transition["condition"] = condition_to_json(condition);
            }
            transitions.push(transition);
        }

        let config_value = json!({
            "agents": agents,
            "tools": [],
            "flow": {
                "name": workflow_name,
                "start": start_node_id,
                "parameters": workflow_config.parameters,
                "variables": workflow_config.variables,
                "nodes": nodes,
                "transitions": transitions
            }
        });

        crate::flow::loader::load_workflow_from_value(&config_value)
    }
}

/// 将 Condition 转换为 JSON 格式(用于 GraphFlow)
fn condition_to_json(condition: &Condition) -> Value {
    let mut cond_json = json!({
        "type": condition.condition_type
    });
    
    if let Some(obj) = condition.params.as_object() {
        for (key, value) in obj {
            cond_json[key] = value.clone();
        }
    } else {
        cond_json["params"] = condition.params.clone();
    }
    
    cond_json
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_condition_to_json() {
        let condition = Condition {
            condition_type: "state_equals".to_string(),
            params: json!({
                "key": "route",
                "value": "a"
            }),
        };

        let json = condition_to_json(&condition);
        assert_eq!(json["type"], "state_equals");
        assert_eq!(json["key"], "route");
        assert_eq!(json["value"], "a");
    }
}
