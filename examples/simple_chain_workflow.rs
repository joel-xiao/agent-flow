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

    println!("Simple Chain Workflow Example");
    println!("{}", "=".repeat(60));
    println!("\nUser input: {}", user_input);
    println!("\nStarting processing...\n");

    let config = json!({
        "agents": [
            {
                "name": "ingest",
                "driver": "qwen",
                "role": "Data Entry Assistant",
                "prompt": "You are a professional data entry assistant, good at understanding user intent and generating structured data.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "capture"
            },
            {
                "name": "planner",
                "driver": "qwen",
                "role": "Planning Specialist",
                "prompt": "You are a professional planning specialist, good at completing and optimizing plans based on contextual information.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "augment"
            },
            {
                "name": "finalizer",
                "driver": "qwen",
                "role": "Summary Generator",
                "prompt": "You are a professional summary generator, good at organizing information into clear final summaries.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "compile"
            }
        ],
        "tools": [],
        "flow": {
            "name": "simple_chain_flow",
            "start": "ingest",
            "nodes": [
                { "kind": "agent", "name": "ingest", "agent": "ingest" },
                { "kind": "agent", "name": "planner", "agent": "planner" },
                { "kind": "agent", "name": "finalizer", "agent": "finalizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "ingest", "to": "planner" },
                { "from": "planner", "to": "finalizer" },
                { "from": "finalizer", "to": "finish" }
            ]
        }
    });

    let bundle: WorkflowBundle = load_workflow_from_value(&config)?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "User",
        "goal": user_input,
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("ingest".to_string()))?;

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
            println!("\nFinal summary:");
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
