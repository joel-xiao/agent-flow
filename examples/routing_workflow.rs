//! 路由工作流示例
//! 
//! 演示如何使用 Decision 节点实现条件路由
//! 工作流：Agent -> Decision -> (Route A | Route B) -> Join -> Terminal

use agentflow::{GraphConfig, FlowContext, FlowExecutor, MessageRole, StructuredMessage};
use agentflow::state::MemoryStore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建路由工作流配置
    let config_json = r#"
    {
      "name": "routing_workflow_example",
      "nodes": [
        {
          "id": "service_qwen",
          "type": "service",
          "config": {
            "name": "qwen",
            "service_type": "llm",
            "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "api_key": "${QWEN_API_KEY}"
          }
        },
        {
          "id": "agent_analyzer",
          "type": "agent",
          "config": {
            "name": "analyzer",
            "driver": "qwen",
            "role": "Requirement Analyst",
            "prompt": "Analyze user requirements and set route label in state",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "analyze",
            "service": "service_qwen"
          }
        },
        {
          "id": "agent_route_a_handler",
          "type": "agent",
          "config": {
            "name": "route_a_handler",
            "driver": "qwen",
            "role": "Route A Handler",
            "prompt": "Handle Route A tasks",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "handle_a",
            "service": "service_qwen"
          }
        },
        {
          "id": "agent_route_b_handler",
          "type": "agent",
          "config": {
            "name": "route_b_handler",
            "driver": "qwen",
            "role": "Route B Handler",
            "prompt": "Handle Route B tasks",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "handle_b",
            "service": "service_qwen"
          }
        },
        {
          "id": "node_analyzer",
          "type": "agent_node",
          "config": {
            "agent": "agent_analyzer"
          },
          "workflow": "workflow_routing"
        },
        {
          "id": "node_decision",
          "type": "decision_node",
          "config": {
            "policy": "first_match",
            "branches": [
              {
                "target": "node_route_a",
                "condition": {
                  "type": "state_equals",
                  "key": "route",
                  "value": "a"
                }
              },
              {
                "target": "node_route_b",
                "condition": {
                  "type": "state_equals",
                  "key": "route",
                  "value": "b"
                }
              }
            ]
          },
          "workflow": "workflow_routing"
        },
        {
          "id": "node_route_a",
          "type": "agent_node",
          "config": {
            "agent": "agent_route_a_handler"
          },
          "workflow": "workflow_routing"
        },
        {
          "id": "node_route_b",
          "type": "agent_node",
          "config": {
            "agent": "agent_route_b_handler"
          },
          "workflow": "workflow_routing"
        },
        {
          "id": "workflow_routing",
          "type": "workflow",
          "config": {
            "name": "routing_flow",
            "start": "node_analyzer"
          }
        }
      ],
      "edges": [
        {
          "from": "node_analyzer",
          "to": "node_decision",
          "type": "always",
          "workflow": "workflow_routing"
        },
        {
          "from": "node_decision",
          "to": "node_route_a",
          "type": "conditional",
          "condition": {
            "type": "state_equals",
            "key": "route",
            "value": "a"
          },
          "workflow": "workflow_routing"
        },
        {
          "from": "node_decision",
          "to": "node_route_b",
          "type": "conditional",
          "condition": {
            "type": "state_equals",
            "key": "route",
            "value": "b"
          },
          "workflow": "workflow_routing"
        }
      ]
    }
    "#;

    let graph_config = GraphConfig::from_json(config_json)?;
    graph_config.validate()?;
    
    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_routing")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 创建初始消息
    let initial_message = StructuredMessage::new(serde_json::json!({
        "user": "User",
        "request": "I need help with route A",
        "route": "a"
    }))
    .into_agent_message(MessageRole::User, "client", Some("node_analyzer".to_string()))?;

    // 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    println!("Workflow completed: {}", result.flow_name);
    Ok(())
}

