//! 食物识别分析应用示例
//!
//! 完整演示食物识别、分量分析和卡路里计算功能
//! 使用所有 AgentFlow 功能：自动路由、决策节点、Join 节点、并行处理

use agentflow::state::MemoryStore;
use agentflow::{FlowContext, FlowExecutor, GraphConfig, MessageRole, StructuredMessage};
use base64::engine::Engine;
use std::fs;
use std::io::{self, Write};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config_json = fs::read_to_string("configs/graph_config_food_analysis.json")?;
    let graph_config = GraphConfig::from_json(&config_json)?;

    // 验证配置
    graph_config.validate()?;

    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_food_analysis")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 读取本地图片文件并转换为 base64
    let image_path = "tests/test_food.jpg";
    let image_base64 = if std::path::Path::new(image_path).exists() {
        let image_data = fs::read(image_path)?;
        Some(base64::engine::general_purpose::STANDARD.encode(&image_data))
    } else {
        println!("⚠️  图片文件不存在: {}, 将使用空图片", image_path);
        None
    };

    // 创建初始消息（包含图片信息）
    let initial_message = StructuredMessage::new(serde_json::json!({
        "user": "User",
        "goal": "Analyze food in this image",
        "image_path": image_path,
        "image_base64": image_base64,
        "steps": []
    }))
    .into_agent_message(
        MessageRole::User,
        "client",
        Some("agent_image_preprocessor".to_string()),
    )?;

    println!("📷 使用图片: {}", image_path);
    if image_base64.is_some() {
        println!(
            "✅ 图片已加载 (Base64 长度: {} 字符)",
            image_base64.as_ref().unwrap().len()
        );
    }

    println!("\n{}", "=".repeat(80));
    println!("🚀 开始执行高级食物分析工作流...");
    println!("✨ 特性演示: 循环重试 | 智能路由 | 并行分析");
    println!("💡 LLM 响应将实时流式输出到终端");
    println!("{}", "=".repeat(80));

    // 确保 stdout/stderr 立即输出，不被缓冲
    io::stdout().flush().unwrap();
    io::stderr().flush().unwrap();

    // 执行工作流（流式输出已在 LlmCaller 中实现）
    eprintln!("\n⏳ 正在启动工作流执行...\n");
    io::stderr().flush().unwrap();

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    println!("\n{}", "=".repeat(80));
    println!("\n✅ 工作流执行完成!");
    println!("📋 工作流名称: {}", result.flow_name);
    println!("📍 最后执行的节点: {}", result.last_node);
    if !result.errors.is_empty() {
        println!("⚠️  执行过程中的错误 ({}):", result.errors.len());
        for (idx, error) in result.errors.iter().enumerate() {
            println!("  {}. {:?}", idx + 1, error);
        }
    } else {
        println!("✅ 没有错误");
    }

    // 显示最终消息状态
    if let Some(ref msg) = result.last_message {
        println!("📨 最终消息: 有内容 ({} 字符)", msg.content.len());
    } else {
        println!("⚠️  最终消息: 无");
    }
    println!();
    std::io::stdout().flush().unwrap();

    // 获取最终结果
    if let Some(final_message) = result.last_message {
        println!("\n📊 最终分析结果:");
        println!("{}", "─".repeat(80));

        // 获取并解析最终消息内容
        let content_to_display = &final_message.content;

        // 尝试解析为食物分析结果 JSON
        if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(content_to_display) {
            // 1. 显示识别到的食物
            if let Some(foods) = result_json.get("foods") {
                println!("\n🍽️  识别到的食物:");
                if let Some(foods_array) = foods.as_array() {
                    for (idx, food) in foods_array.iter().enumerate() {
                        if let Some(name) = food.get("name").and_then(|v| v.as_str()) {
                            print!("  {}. {}", idx + 1, name);
                            if let Some(confidence) =
                                food.get("confidence").and_then(|v| v.as_f64())
                            {
                                print!(" (置信度: {:.1}%)", confidence * 100.0);
                            }
                            println!();
                        }
                    }
                }
            }

            // 2. 显示营养信息
            if let Some(summary) = result_json.get("summary") {
                if let Some(total_calories) = summary.get("total_calories").and_then(|v| v.as_f64())
                {
                    println!("\n🔥 总卡路里: {} kcal", total_calories);
                }
                if let Some(total_foods) = summary.get("total_foods").and_then(|v| v.as_u64()) {
                    println!("📦 食物数量: {}", total_foods);
                }
                if let Some(confidence) = summary.get("confidence_score").and_then(|v| v.as_f64()) {
                    println!("🎯 整体置信度: {:.1}%", confidence * 100.0);
                }
            }

            // 3. 显示推荐信息
            if let Some(recommendations) =
                result_json.get("recommendations").and_then(|v| v.as_str())
            {
                println!("\n💡 健康建议:");
                println!("  {}", recommendations);
            }

            // 4. 显示完整 JSON (如果上述字段都不存在)
            if result_json.get("foods").is_none() && result_json.get("summary").is_none() {
                println!("\n📄 完整 JSON 结果:");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result_json)
                        .unwrap_or_else(|_| content_to_display.clone())
                );
            }
        } else {
            // 如果无法解析为 JSON，直接显示原始内容
            println!("\n📄 最终响应内容:");
            println!("{}", content_to_display);

            if content_to_display.trim().is_empty() {
                println!("⚠️  警告: 最终消息内容为空");
            }
        }

        println!("\n{}", "─".repeat(80));
    } else {
        println!("⚠️  未获取到最终消息");
    }

    Ok(())
}
