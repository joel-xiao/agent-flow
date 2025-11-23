//! 自动路由工作流示例
//! 
//! 演示如何使用自动路由功能（LLM 驱动的路由）
//! 工作流：Router Agent -> (自动路由到多个目标) -> Join -> Terminal

use agentflow::{GraphConfig, FlowContext, FlowExecutor, MessageRole, StructuredMessage};
use agentflow::state::MemoryStore;
use std::sync::Arc;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config_json = fs::read_to_string("configs/graph_config_auto_routing.json")?;
    let graph_config = GraphConfig::from_json(&config_json)?;
    
    // 验证配置
    graph_config.validate()?;
    
    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_auto_routing")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 创建初始消息
    let initial_message = StructuredMessage::new(serde_json::json!({
        "user": "VIP Customer",
        "request": "I need urgent help with my premium account",
        "priority": "high"
    }))
    .into_agent_message(MessageRole::User, "client", Some("node_router".to_string()))?;

    // 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    println!("Workflow completed: {}", result.flow_name);
    Ok(())
}

