use std::sync::Arc;
use serde_json::Value;

use crate::agent::{AgentRegistry, register_agent};
use crate::error::{AgentFlowError, Result};
use crate::flow::{
    DecisionBranch, DecisionPolicy, Flow, FlowBuilder, JoinStrategy,
};
use crate::tools::ToolRegistry;

use crate::flow::config::{GraphFlow, GraphNode, WorkflowConfig};
use crate::flow::agent::{ConfigDrivenAgent, ConfigDrivenTool};
use crate::flow::services::llm_client_factory::LlmClientFactory;

/// 工作流包，包含流程、Agent 注册表和工具注册表
pub struct WorkflowBundle {
    pub flow: Flow,
    pub agents: AgentRegistry,
    pub tools: ToolRegistry,
}

/// 从 GraphFlow 构建 Flow
pub fn build_flow_from_graph(graph: &GraphFlow) -> Flow {
    let mut builder = FlowBuilder::new(graph.name.clone());
    builder.set_start(&graph.start);

    for parameter in graph.parameters.clone() {
        builder.with_parameter(parameter.into_flow_param());
    }

    for variable in graph.variables.clone() {
        builder.declare_variable(variable.into_flow_variable());
    }

    for node in &graph.nodes {
        match node {
            GraphNode::Agent { name, agent } => {
                builder.add_agent_node(name, agent);
            }
            GraphNode::Decision {
                name,
                policy,
                branches,
            } => {
                let policy = match policy.as_deref() {
                    Some("all_matches") => DecisionPolicy::AllMatches,
                    _ => DecisionPolicy::FirstMatch,
                };
                let branches = branches
                    .iter()
                    .map(|branch| DecisionBranch {
                        name: branch.name.clone(),
                        condition: branch.condition.as_ref().map(|c| c.build()),
                        target: branch.target.clone(),
                    })
                    .collect::<Vec<_>>();
                builder.add_decision_node(name, policy, branches);
            }
            GraphNode::Join {
                name,
                strategy,
                inbound,
            } => {
                let strategy = match strategy.as_str() {
                    "any" => JoinStrategy::Any,
                    other => {
                        if other.starts_with("count:") {
                            let parts: Vec<_> = other.split(':').collect();
                            let count = parts
                                .get(1)
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1);
                            JoinStrategy::Count(count)
                        } else {
                            JoinStrategy::All
                        }
                    }
                };
                builder.add_join_node(name, strategy, inbound.clone());
            }
            GraphNode::Loop {
                name,
                entry,
                condition,
                max_iterations,
                exit,
            } => {
                let continuation = condition.as_ref().map(|c| c.build());
                builder.add_loop_node(name, entry, continuation, *max_iterations, exit.clone());
            }
            GraphNode::Tool { name, pipeline } => {
                builder.add_tool_node(name, pipeline);
            }
            GraphNode::Terminal { name } => {
                builder.add_terminal_node(name);
            }
        }
    }

    for transition in &graph.transitions {
        if let Some(condition) = &transition.condition {
            builder.connect_if_named(
                &transition.from,
                &transition.to,
                transition.name.clone(),
                condition.build(),
            );
        } else if let Some(name) = &transition.name {
            builder.connect_named(&transition.from, &transition.to, Some(name.clone()));
        } else {
            builder.connect(&transition.from, &transition.to);
        }
    }

    builder.build()
}

/// 从 JSON Value 加载工作流
pub fn load_workflow_from_value(value: &Value) -> Result<WorkflowBundle> {
    let config: WorkflowConfig = serde_json::from_value(value.clone())
        .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;

    let mut agents = AgentRegistry::new();
    for profile in &config.agents {
        // 使用 LlmClientFactory 创建客户端，统一处理所有逻辑
        let llm_client = LlmClientFactory::create_client(profile)?;

        let agent = ConfigDrivenAgent {
            profile: Arc::new(profile.clone()),
            name: Box::leak(profile.name.clone().into_boxed_str()),
            #[cfg(feature = "openai-client")]
            llm_client,
        };
        register_agent(&profile.name, Arc::new(agent), &mut agents);
    }

    let mut tools = ToolRegistry::new();
    for profile in &config.tools {
        let tool = ConfigDrivenTool {
            profile: Arc::new(profile.clone()),
            name: Box::leak(profile.name.clone().into_boxed_str()),
        };
        tools.register(Arc::new(tool));
    }

    let flow = build_flow_from_graph(&config.flow);

    Ok(WorkflowBundle {
        flow,
        agents,
        tools,
    })
}

/// 从 JSON 字符串加载工作流
pub fn load_workflow_from_str(config: &str) -> Result<WorkflowBundle> {
    let value: Value =
        serde_json::from_str(config).map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
    load_workflow_from_value(&value)
}

