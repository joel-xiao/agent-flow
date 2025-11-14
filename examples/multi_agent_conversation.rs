use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    FlowContext, FlowExecutor, MessageRole, StructuredMessage, WorkflowBundle,
    load_workflow_from_value,
};
use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("QWEN_API_KEY")?;
    let user_input = std::env::args().nth(1).ok_or_else(|| anyhow::anyhow!("Missing user input parameter"))?;

    println!("Multi-Role Multi-Agent Conversation Example");
    println!("{}", "=".repeat(60));
    println!("\nUser requirement: {}", user_input);
    println!("\nStarting multi-role discussion...\n");

    let config = json!({
        "agents": [
            {
                "name": "product_manager",
                "driver": "qwen",
                "role": "Product Manager",
                "prompt": "You are an experienced product manager, good at analyzing user needs, designing product features, and creating product roadmaps. Based on user requirements, propose clear product solutions including core features, user value, and priorities.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "product_analysis"
            },
            {
                "name": "tech_expert",
                "driver": "qwen",
                "role": "Technical Expert",
                "prompt": "You are a senior technical expert, good at technical architecture design, technology selection, and feasibility assessment. Based on the product solution, evaluate technical feasibility, propose technical architecture recommendations, and identify technical risks and challenges.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "tech_evaluation"
            },
            {
                "name": "designer",
                "driver": "qwen",
                "role": "UI/UX Designer",
                "prompt": "You are an excellent UI/UX designer, good at user experience design, interface design, and interaction design. Based on the product and technical solutions, propose design recommendations including user interface design, interaction flow, and visual style.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "design_proposal"
            },
            {
                "name": "project_manager",
                "driver": "qwen",
                "role": "Project Manager",
                "prompt": "You are a professional project manager, good at project planning, resource coordination, and risk management. Based on the product, technical, and design solutions, create a detailed project implementation plan including timeline, personnel allocation, and milestones.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "project_planning"
            },
            {
                "name": "summarizer",
                "driver": "qwen",
                "role": "Meeting Recorder",
                "prompt": "You are a professional meeting recorder, good at organizing and summarizing multi-party discussion content. Organize the discussions from the product manager, technical expert, designer, and project manager into a clear and complete summary report including requirement analysis, solution design, and implementation plan.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "summarize"
            }
        ],
        "tools": [],
        "flow": {
            "name": "multi_agent_conversation_flow",
            "start": "product_manager",
            "nodes": [
                { "kind": "agent", "name": "product_manager", "agent": "product_manager" },
                { "kind": "agent", "name": "tech_expert", "agent": "tech_expert" },
                { "kind": "agent", "name": "designer", "agent": "designer" },
                { "kind": "agent", "name": "project_manager", "agent": "project_manager" },
                { "kind": "agent", "name": "summarizer", "agent": "summarizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "product_manager", "to": "tech_expert" },
                { "from": "tech_expert", "to": "designer" },
                { "from": "designer", "to": "project_manager" },
                { "from": "project_manager", "to": "summarizer" },
                { "from": "summarizer", "to": "finish" }
            ]
        }
    });

    let bundle: WorkflowBundle = load_workflow_from_value(&config)?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "ProductOwner",
        "goal": user_input,
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("product_manager".to_string()))?;

    let result = executor.start(ctx, initial_message).await?;

    println!("\n{}", "=".repeat(60));
    println!("Workflow execution completed!");
    println!("   Flow name: {}", result.flow_name);
    println!("   Last node: {}", result.last_node);

    if let Some(message) = result.last_message {
        let payload: serde_json::Value = serde_json::from_str(&message.content)?;
        
        if let Some(steps) = payload.get("steps").and_then(|s| s.as_array()) {
            println!("\nExecution steps ({} total):", steps.len());
            for (i, step) in steps.iter().enumerate() {
                let agent = step["agent"].as_str().ok_or_else(|| anyhow::anyhow!("Missing agent field"))?;
                let intent = step["intent"].as_str().ok_or_else(|| anyhow::anyhow!("Missing intent field"))?;
                println!("   {}. {} ({})", i + 1, agent, intent);
            }
        }

        if let Some(response) = payload.get("response").and_then(|r| r.as_str()) {
            println!("\nFinal summary report:");
            println!("{}", response);
        }
    }

    if !result.errors.is_empty() {
        println!("\n{} errors occurred during execution:", result.errors.len());
        for error in &result.errors {
            println!("   - {:?}", error);
        }
    }

    Ok(())
}
