use serde_json::{json, Value};
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use crate::flow::loader::WorkflowBundle;
use super::graph::{GraphConfig, GraphNode};
use super::nodes::{ServiceConfig, AgentNodeConfig, DecisionNodeConfig, JoinNodeConfig, WorkflowConfig};
use super::agent_config::AgentConfig;
use super::conditions::Condition;

/// 从 GraphConfig 加载工作流
impl GraphConfig {
    /// 加载指定的工作流
    pub fn load_workflow(&self, workflow_id: &str) -> Result<WorkflowBundle> {
        // 验证配置
        self.validate()?;

        // 获取 workflow 节点
        let workflow_node = self.get_node(workflow_id)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("Workflow '{}' not found", workflow_id)))?;

        let workflow_config = workflow_node.as_workflow()?;
        let workflow_name = workflow_config.name.clone();
        let start_node_id = workflow_config.start.clone();

        // 获取属于该工作流的所有节点
        let workflow_nodes: Vec<&GraphNode> = self.get_nodes_by_workflow(workflow_id);

        // 收集所有被引用的 agent ID
        let mut agent_ids_used = std::collections::HashSet::new();
        for node in &workflow_nodes {
            if node.node_type == "agent_node" {
                if let Ok(agent_node_config) = node.as_agent_node() {
                    agent_ids_used.insert(agent_node_config.agent.clone());
                }
            }
        }

        // 收集所有被引用的 agent 配置
        let agents: Vec<Value> = self.get_agents()
            .iter()
            .filter(|agent_node| agent_ids_used.contains(&agent_node.id))
            .filter_map(|agent_node| {
                agent_node.as_agent().ok().map(|agent_config| {
                    // 查找对应的 service 节点
                    let service_config = self.get_services()
                        .iter()
                        .find(|service_node| service_node.id == agent_config.service)
                        .and_then(|service_node| service_node.as_service().ok());
                    
                    // 合并 endpoint：优先 agent 配置，否则使用 service 的 base_url
                    let endpoint = agent_config.endpoint.clone()
                        .or_else(|| service_config.as_ref().map(|s| s.base_url.clone()));
                    
                    // 合并 api_key：优先 agent 配置，否则使用 service 的 api_key
                    let api_key = agent_config.api_key.clone()
                        .or_else(|| service_config.as_ref().map(|s| s.api_key.clone()));
                    
                    let mut agent_json = json!({
                        "name": agent_config.name,
                        "driver": agent_config.driver,
                        "role": agent_config.role,
                        "prompt": agent_config.prompt,
                        "model": agent_config.model,
                        "intent": agent_config.intent,
                        "service": agent_config.service
                    });
                    
                    // 添加合并后的 endpoint 和 api_key
                    if let Some(endpoint) = endpoint {
                        agent_json["endpoint"] = json!(endpoint);
                    }
                    if let Some(api_key) = api_key {
                        agent_json["api_key"] = json!(api_key);
                    }
                    if let Some(metadata) = &agent_config.metadata {
                        agent_json["metadata"] = metadata.clone();
                    }
                    
                    // 添加路由相关字段（如果存在）
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
                    
                    // 添加业务规则（如果存在）
                    if let Some(rules) = &agent_config.rules {
                        agent_json["rules"] = serde_json::to_value(rules)
                            .unwrap_or_else(|_| json!({}));
                    }
                    
                    agent_json
                })
            })
            .collect();

        // 构建 GraphFlow 格式的节点
        let mut nodes = Vec::new();
        for node in &workflow_nodes {
            match node.node_type.as_str() {
                "agent_node" => {
                    if let Ok(agent_node_config) = node.as_agent_node() {
                        // 查找被引用的 agent 节点
                        if let Some(agent_node) = self.get_node(&agent_node_config.agent) {
                            if let Ok(agent_config) = agent_node.as_agent() {
                                nodes.push(json!({
                                    "kind": "agent",
                                    "name": node.id,
                                    "agent": agent_config.name
                                }));
                            }
                        }
                    }
                }
                "decision_node" => {
                    if let Ok(decision_config) = node.as_decision_node() {
                        let branches: Vec<Value> = decision_config.branches
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
                _ => {
                    // 忽略其他类型的节点
                }
            }
        }

        // 构建 transitions (从 edges 中提取)
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

        // 构建最终的配置值
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

        // 使用现有的 loader 加载工作流
        crate::flow::loader::load_workflow_from_value(&config_value)
    }
}

/// 将 Condition 转换为 JSON 格式(用于 GraphFlow)
fn condition_to_json(condition: &Condition) -> Value {
    let mut cond_json = json!({
        "type": condition.condition_type
    });
    
    // 将 params 中的字段合并到根对象
    if let Some(obj) = condition.params.as_object() {
        for (key, value) in obj {
            cond_json[key] = value.clone();
        }
    } else {
        // 如果 params 不是对象,直接使用它
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

