//! Loop 节点工作流示例
//! 
//! 演示如何使用 Loop 节点实现循环处理
//! 工作流：Entry -> Loop (Condition) -> Exit

use agentflow::{GraphConfig, FlowContext, FlowExecutor, MessageRole, StructuredMessage};
use agentflow::state::MemoryStore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Loop 工作流配置
    let config_json = r#"
    {
      "name": "loop_workflow_example",
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
          "id": "agent_processor",
          "type": "agent",
          "config": {
            "name": "processor",
            "driver": "qwen",
            "role": "Task Processor",
            "prompt": "Process one item from the queue",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "process",
            "service": "service_qwen"
          }
        },
        {
          "id": "node_processor",
          "type": "agent_node",
          "config": {
            "agent": "agent_processor"
          },
          "workflow": "workflow_loop"
        },
        {
          "id": "node_loop",
          "type": "loop_node",
          "config": {
            "entry": "node_processor",
            "condition": {
              "type": "state_equals",
              "key": "queue_empty",
              "value": "false"
            },
            "max_iterations": 10
          },
          "workflow": "workflow_loop"
        },
        {
          "id": "workflow_loop",
          "type": "workflow",
          "config": {
            "name": "loop_flow",
            "start": "node_loop"
          }
        }
      ],
      "edges": [
        {
          "from": "node_loop",
          "to": "node_processor",
          "type": "always",
          "workflow": "workflow_loop"
        }
      ]
    }
    "#;

    let graph_config = GraphConfig::from_json(config_json)?;
    graph_config.validate()?;
    
    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_loop")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 创建初始消息
    let initial_message = StructuredMessage::new(serde_json::json!({
        "user": "User",
        "queue": ["task1", "task2", "task3"]
    }))
    .into_agent_message(MessageRole::User, "client", Some("node_loop".to_string()))?;

    // 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    println!("Workflow completed: {}", result.flow_name);
    Ok(())
}

