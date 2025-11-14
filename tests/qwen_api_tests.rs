use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    FlowContext, FlowExecutor, MessageRole, StructuredMessage, WorkflowBundle,
    load_workflow_from_value,
};
use anyhow::Result;
use serde_json::json;

fn get_test_api_key() -> String {
    std::env::var("QWEN_API_KEY")
        .expect("QWEN_API_KEY environment variable not set")
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn qwen_api_workflow_executes_with_real_llm() -> Result<()> {
    let api_key = get_test_api_key();
    let config = json!({
        "agents": [
            {
                "name": "assistant",
                "driver": "qwen",
                "role": "AI Assistant",
                "prompt": "You are a professional AI assistant, able to understand user needs and provide help.",
                "model": "qwen-turbo",
                "api_key": api_key,
                "intent": "assist"
            }
        ],
        "tools": [],
        "flow": {
            "name": "qwen_test_flow",
            "start": "assistant",
            "nodes": [
                { "kind": "agent", "name": "assistant", "agent": "assistant" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "assistant", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_workflow_from_value(&config)?;

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "TestUser",
        "goal": "Please introduce yourself in one sentence",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("assistant".to_string()))?;

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "qwen_test_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: serde_json::Value = serde_json::from_str(&final_message.content)?;
    
    assert!(payload.get("response").is_some(), "Should contain LLM response");
    let response = payload["response"].as_str().unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(!response.contains("[LLM错误"), "Should not contain LLM error");
    
    println!("Qwen API response: {}", response);

    Ok(())
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn qwen_api_complex_workflow_with_multiple_agents() -> Result<()> {
    let api_key = get_test_api_key();
    let config = json!({
        "agents": [
            {
                "name": "analyzer",
                "driver": "qwen",
                "role": "Requirement Analyst",
                "prompt": "You are good at analyzing user requirements and extracting key information.",
                "model": "qwen-turbo",
                "api_key": api_key,
                "intent": "analyze"
            },
            {
                "name": "planner",
                "driver": "qwen",
                "role": "Planning Specialist",
                "prompt": "You are good at creating detailed execution plans based on requirements.",
                "model": "qwen-turbo",
                "api_key": api_key,
                "intent": "plan"
            },
            {
                "name": "executor",
                "driver": "qwen",
                "role": "Executor",
                "prompt": "You are good at executing plans and generating final results.",
                "model": "qwen-turbo",
                "api_key": api_key,
                "intent": "execute"
            }
        ],
        "tools": [],
        "flow": {
            "name": "qwen_complex_flow",
            "start": "analyzer",
            "nodes": [
                { "kind": "agent", "name": "analyzer", "agent": "analyzer" },
                { "kind": "agent", "name": "planner", "agent": "planner" },
                { "kind": "agent", "name": "executor", "agent": "executor" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "analyzer", "to": "planner" },
                { "from": "planner", "to": "executor" },
                { "from": "executor", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_workflow_from_value(&config)?;

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "Developer",
        "goal": "Please help me create a plan for learning Rust programming, including 3 main steps",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("analyzer".to_string()))?;

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "qwen_complex_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: serde_json::Value = serde_json::from_str(&final_message.content)?;
    
    let steps = payload["steps"].as_array().expect("steps array");
    assert_eq!(steps.len(), 3, "Should have 3 agents executed");
    
    assert!(payload.get("response").is_some(), "Should contain final response");
    let response = payload["response"].as_str().unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(!response.contains("[LLM错误"), "Should not contain LLM error");
    
    println!("Complete workflow execution result:");
    println!("Steps: {}", steps.len());
    for (i, step) in steps.iter().enumerate() {
        println!("  Step{}: agent={}, intent={}", 
            i + 1, 
            step["agent"].as_str().unwrap_or("unknown"),
            step["intent"].as_str().unwrap_or("unknown")
        );
    }
    println!("Final response: {}", response);

    Ok(())
}
