//! LegalFlow - 智能法律案件评估与文书生成系统
//!
//! 展示功能：
//! 1. 复杂图编排 (Graph Orchestration)
//! 2. 多角色专业分工 (Multi-Agent Role)
//! 3. 自动路由与分支 (Auto-Routing)
//! 4. 循环审核机制 (Loop & Quality Check)
//! 5. 状态共享与上下文注入 (State Store & Context Injection)
//! 6. 工具调用 (Image Gen & Download)
//! 7. 并行处理 (Parallel Processing)

use agentflow::state::MemoryStore;
use agentflow::tools::{ToolOrchestrator, ToolPipeline, ToolStep, ToolStrategy};
use agentflow::{FlowContext, FlowExecutor, GraphConfig, MessageRole, StructuredMessage};
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置
    let config_path = "configs/graph_config_legal_flow.json";
    println!("⚖️  正在启动 LegalFlow 智能律所系统...");
    println!("📂 加载配置: {}", config_path);

    let config_json = fs::read_to_string(config_path)?;
    let graph_config = GraphConfig::from_json(&config_json)?;
    graph_config.validate()?;

    // 2. 获取 Workflow ID
    let config_value: serde_json::Value = serde_json::from_str(&config_json)?;
    let workflow_id = config_value["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["type"] == "workflow"))
        .and_then(|n| n["id"].as_str())
        .ok_or("No workflow found")?;

    // 3. 初始化执行器
    let bundle = graph_config.load_workflow(workflow_id)?;
    
    // 4. 注册工具 (下载器)
    let mut orchestrator = ToolOrchestrator::new(bundle.tools.clone());
    let download_pipeline = ToolPipeline::new(
        "download_file",
        ToolStrategy::Sequential(vec![
            ToolStep::new("downloader", serde_json::json!({}))
        ])
    );
    orchestrator.register_pipeline(download_pipeline)?;

    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools)
        .with_tool_orchestrator(Arc::new(orchestrator));

    // 5. 模拟案件输入
    // 这是一个复杂的民事纠纷案例
    let case_statement = serde_json::json!({
        "client_name": "John Doe",
        "incident_date": "2023-11-15",
        "statement": "I hired 'Reliable Construction Inc.' to renovate my kitchen. We signed a contract for $50,000. I paid a $25,000 deposit upfront. They were supposed to finish by Dec 2023. It is now Nov 2024, and they only demolished the cabinets and left. They refuse to answer my calls or refund the money. I want my money back and damages for the delay."
    });

    // 将案件详情合并到 goal 字段，确保被首个 Agent (Intake Specialist) 准确识别
    let goal_prompt = format!(
        "START_LEGAL_INTAKE_WORKFLOW\n\nCASE DATA:\n{}", 
        case_statement.to_string()
    );

    let input_data = serde_json::json!({
        "user": "Legal Clerk",
        "goal": goal_prompt,
        "steps": []
    });

    let start_node = config_value["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["type"] == "workflow"))
        .and_then(|n| n["config"]["start"].as_str())
        .ok_or("No start node")?;

    let initial_message = StructuredMessage::new(input_data)
        .into_agent_message(MessageRole::User, "user", Some(start_node.to_string()))?;

    // 6. 执行工作流
    println!("\n🚀 案件受理中... Workflow ID: {}", workflow_id);
    println!("📄 案件摘要: 客户支付了装修定金，但承包商未履行合同且拒绝退款。\n");
    
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    // 7. 输出结果
    println!("\n{}", "=".repeat(50));
    println!("✅ 案件处理完成: {}", result.flow_name);
    println!("{}", "=".repeat(50));

    if let Some(msg) = &result.last_message {
        println!("\n📁 最终案卷 (Case File):\n");
        println!("{}", msg.content);
    }

    // 验证生成的证据文件
    let evidence_dir = "legal_evidence";
    if let Ok(entries) = fs::read_dir(evidence_dir) {
        println!("\n🖼️  生成的法庭证据可视化文件:");
        for entry in entries {
            if let Ok(entry) = entry {
                println!("   - {:?}", entry.path());
            }
        }
    }

    Ok(())
}

