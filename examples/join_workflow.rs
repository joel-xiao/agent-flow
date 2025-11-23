//! Join 节点工作流示例
//! 
//! 演示如何使用 Join 节点实现并行处理和结果合并
//! 工作流：Splitter -> (Worker A, Worker B) -> Join -> Terminal

use agentflow::{GraphConfig, FlowContext, FlowExecutor, MessageRole, StructuredMessage};
use agentflow::state::MemoryStore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Join 工作流配置
    let config_json = r#"
    {
      "name": "join_workflow_example",
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
          "id": "agent_splitter",
          "type": "agent",
          "config": {
            "name": "splitter",
            "driver": "qwen",
            "role": "Task Splitter",
            "prompt": "Split the task into parts",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "split",
            "service": "service_qwen"
          }
        },
        {
          "id": "agent_worker_a",
          "type": "agent",
          "config": {
            "name": "worker_a",
            "driver": "qwen",
            "role": "Worker A",
            "prompt": "Process part A",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "work_a",
            "service": "service_qwen"
          }
        },
        {
          "id": "agent_worker_b",
          "type": "agent",
          "config": {
            "name": "worker_b",
            "driver": "qwen",
            "role": "Worker B",
            "prompt": "Process part B",
            "model": "qwen-max",
            "api_key": "${QWEN_API_KEY}",
            "intent": "work_b",
            "service": "service_qwen"
          }
        },
        {
          "id": "node_splitter",
          "type": "agent_node",
          "config": {
            "agent": "agent_splitter"
          },
          "workflow": "workflow_join"
        },
        {
          "id": "node_worker_a",
          "type": "agent_node",
          "config": {
            "agent": "agent_worker_a"
          },
          "workflow": "workflow_join"
        },
        {
          "id": "node_worker_b",
          "type": "agent_node",
          "config": {
            "agent": "agent_worker_b"
          },
          "workflow": "workflow_join"
        },
        {
          "id": "node_join",
          "type": "join_node",
          "config": {
            "strategy": "all",
            "inbound": ["node_worker_a", "node_worker_b"]
          },
          "workflow": "workflow_join"
        },
        {
          "id": "workflow_join",
          "type": "workflow",
          "config": {
            "name": "join_flow",
            "start": "node_splitter"
          }
        }
      ],
      "edges": [
        {
          "from": "node_splitter",
          "to": "node_worker_a",
          "type": "always",
          "workflow": "workflow_join"
        },
        {
          "from": "node_splitter",
          "to": "node_worker_b",
          "type": "always",
          "workflow": "workflow_join"
        },
        {
          "from": "node_worker_a",
          "to": "node_join",
          "type": "always",
          "workflow": "workflow_join"
        },
        {
          "from": "node_worker_b",
          "to": "node_join",
          "type": "always",
          "workflow": "workflow_join"
        }
      ]
    }
    "#;

    let graph_config = GraphConfig::from_json(config_json)?;
    graph_config.validate()?;
    
    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_join")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 创建初始消息
    let initial_message = StructuredMessage::new(serde_json::json!({
        "user": "User",
        "task": "Process in parallel"
    }))
    .into_agent_message(MessageRole::User, "client", Some("node_splitter".to_string()))?;

    // 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    println!("Workflow completed: {}", result.flow_name);
    Ok(())
}

