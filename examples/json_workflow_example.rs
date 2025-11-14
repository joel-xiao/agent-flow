use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    FlowContext, FlowExecutor, WorkflowBundle,
    load_workflow_from_value,
};
use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("QWEN_API_KEY")?;

    let config = json!({
        "agents": [
            {
                "name": "greeter",
                "driver": "qwen",
                "role": "Greeting Assistant",
                "prompt": "You are a friendly greeting assistant, good at greeting users in Chinese and understanding their needs.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "greet"
            },
            {
                "name": "responder",
                "driver": "qwen",
                "role": "Response Assistant",
                "prompt": "You are a professional response assistant, good at providing useful advice and answers based on user needs.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "respond"
            }
        ],
        "tools": [],
        "flow": {
            "name": "greeting_flow",
            "start": "greeter",
            "nodes": [
                { "kind": "agent", "name": "greeter", "agent": "greeter" },
                { "kind": "agent", "name": "responder", "agent": "responder" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "greeter", "to": "responder" },
                { "from": "responder", "to": "finish" }
            ]
        }
    });

    let bundle: WorkflowBundle = load_workflow_from_value(&config)?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);
    let initial_message = agentflow::AgentMessage::user("Hello, I would like to learn how to use AgentFlow");

    println!("Starting workflow execution...\n");

    let result = executor.start(ctx, initial_message).await?;

    println!("\nWorkflow execution completed!");
    println!("   Flow name: {}", result.flow_name);
    println!("   Last node: {}", result.last_node);

    if let Some(message) = result.last_message {
        println!("\nFinal response:");
        println!("   {}", message.content);
    }

    if !result.errors.is_empty() {
        println!("\n{} errors occurred during execution:", result.errors.len());
        for error in &result.errors {
            println!("   - {:?}", error);
        }
    }

    Ok(())
}
