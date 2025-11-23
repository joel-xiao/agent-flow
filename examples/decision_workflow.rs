//! 决策节点工作流示例
//! 
//! 演示如何使用 Decision 节点实现多分支路由
//! 工作流：Agent -> Decision (FirstMatch/AllMatches) -> Multiple Branches -> Join

use agentflow::{GraphConfig, FlowContext, FlowExecutor, MessageRole, StructuredMessage};
use agentflow::state::MemoryStore;
use std::sync::Arc;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建简单的决策工作流配置
    let config_json = r#"
    {
      "name": "decision_workflow_example",
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
          "id": "agent_classifier",
          "type": "agent",
          "config": {
            "name": "classifier",
            "driver": "qwen",
            "role": "Request Classifier",
            "prompt": "Classify the request and set route label in state",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "classify",
            "service": "service_qwen"
          }
        },
        {
          "id": "node_classifier",
          "type": "agent_node",
          "config": {
            "agent": "agent_classifier"
          },
          "workflow": "workflow_decision"
        },
        {
          "id": "node_decision",
          "type": "decision_node",
          "config": {
            "policy": "first_match",
            "branches": [
              {
                "target": "node_handler_a",
                "condition": {
                  "type": "state_equals",
                  "key": "route",
                  "value": "a"
                }
              },
              {
                "target": "node_handler_b",
                "condition": {
                  "type": "state_equals",
                  "key": "route",
                  "value": "b"
                }
              }
            ]
          },
          "workflow": "workflow_decision"
        },
        {
          "id": "node_handler_a",
          "type": "agent_node",
          "config": {
            "agent": "agent_classifier"
          },
          "workflow": "workflow_decision"
        },
        {
          "id": "node_handler_b",
          "type": "agent_node",
          "config": {
            "agent": "agent_classifier"
          },
          "workflow": "workflow_decision"
        },
        {
          "id": "workflow_decision",
          "type": "workflow",
          "config": {
            "name": "decision_flow",
            "start": "node_classifier"
          }
        }
      ],
      "edges": [
        {
          "from": "node_classifier",
          "to": "node_decision",
          "type": "always",
          "workflow": "workflow_decision"
        },
        {
          "from": "node_decision",
          "to": "node_handler_a",
          "type": "conditional",
          "condition": {
            "type": "state_equals",
            "key": "route",
            "value": "a"
          },
          "workflow": "workflow_decision"
        },
        {
          "from": "node_decision",
          "to": "node_handler_b",
          "type": "conditional",
          "condition": {
            "type": "state_equals",
            "key": "route",
            "value": "b"
          },
          "workflow": "workflow_decision"
        }
      ]
    }
    "#;

    let graph_config = GraphConfig::from_json(config_json)?;
    graph_config.validate()?;
    
    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_decision")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 创建初始消息
    let initial_message = StructuredMessage::new(serde_json::json!({
        "user": "User",
        "request": "Process route A"
    }))
    .into_agent_message(MessageRole::User, "client", Some("node_classifier".to_string()))?;

    // 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    println!("Workflow completed: {}", result.flow_name);
    Ok(())
}

