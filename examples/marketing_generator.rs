//! 营销内容生成系统
//!
//! 完全由 JSON 配置驱动，包括图片生成和下载
//!
//! 使用方法：
//! ```bash
//! cargo run --example marketing_generator --features openai-client
//! ```

use agentflow::state::MemoryStore;
use agentflow::tools::{ToolOrchestrator, ToolPipeline, ToolStep, ToolStrategy};
use agentflow::{FlowContext, FlowExecutor, GraphConfig, MessageRole, StructuredMessage};
use std::env;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "configs/graph_config_marketing_generator.json".to_string());
    
    let config_json = fs::read_to_string(&config_path)?;
    let graph_config = GraphConfig::from_json(&config_json)?;
    graph_config.validate()?;

    // 2. 查找工作流
    let config_value: serde_json::Value = serde_json::from_str(&config_json)?;
    let workflow_id = config_value["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["type"] == "workflow"))
        .and_then(|n| n["id"].as_str())
        .ok_or("No workflow found")?;

    // 3. 加载执行器（内置工具已自动注册）
    let bundle = graph_config.load_workflow(workflow_id)?;
    
    // 4. 创建 ToolOrchestrator 并注册 pipeline
    // 注意：参数现在从 JSON 配置读取，这里只需要注册空 pipeline
    let mut orchestrator = ToolOrchestrator::new(bundle.tools.clone());
    
    // 注册下载 pipeline（参数从 graph JSON 的 tool_node.config.params 读取）
    let download_pipeline = ToolPipeline::new(
        "download_file",
        ToolStrategy::Sequential(vec![
            ToolStep::new("downloader", serde_json::json!({}))  // 空参数，实际参数从 JSON 读取
        ])
    );
    orchestrator.register_pipeline(download_pipeline)?;
    
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools)
        .with_tool_orchestrator(Arc::new(orchestrator));

    // 5. 准备输入数据
    let product_info = serde_json::json!({
            "name": "智能健康手环 ProFit X1",
            "category": "智能穿戴设备",
            "features": ["24小时心率监测", "血氧饱和度检测", "50米防水", "30天超长续航"],
            "price": "¥599",
            "target_market": "健身爱好者"
    });

    let input_data = serde_json::json!({
        "user": "Marketing Team",
        "goal": format!("Generate marketing content for product: {}", product_info),
        "steps": [],
        "product": product_info
    });

    let start_node = config_value["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["type"] == "workflow"))
        .and_then(|n| n["config"]["start"].as_str())
        .ok_or("No start node")?;

    let initial_message = StructuredMessage::new(input_data)
        .into_agent_message(MessageRole::User, "user", Some(start_node.to_string()))?;

    // 6. 执行工作流（JSON 驱动，包含图片生成和下载）
    println!("🚀 执行工作流: {}", workflow_id);
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    // 7. 输出结果
    println!("\n✅ 完成: {}", result.flow_name);
    if let Some(msg) = &result.last_message {
        println!("\n结果:\n{}", msg.content);
    }

    Ok(())
}
