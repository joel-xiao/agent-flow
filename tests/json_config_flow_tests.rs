use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    FlowContext, FlowExecutor, MessageRole, StructuredMessage, WorkflowBundle,
    load_workflow_from_value,
};
use anyhow::Result;
use serde_json::{Value, json};

fn get_test_api_key() -> String {
    std::env::var("QWEN_API_KEY")
        .expect("QWEN_API_KEY environment variable not set")
}

fn load_bundle(config: Value) -> WorkflowBundle {
    load_workflow_from_value(&config).expect("load workflow from json")
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn json_config_driven_flow_executes_chain() -> Result<()> {
    let api_key = get_test_api_key();
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
            "name": "json_chain_flow",
            "start": "ingest",
            "parameters": [
                {
                    "name": "payload",
                    "kind": "input",
                    "type_name": "serde_json::Value",
                    "description": "Initial message"
                }
            ],
            "variables": [
                { "name": "active_lane", "scope": "global" }
            ],
            "nodes": [
                { "kind": "agent", "name": "ingest", "agent": "ingest" },
                { "kind": "agent", "name": "planner", "agent": "planner" },
                { "kind": "agent", "name": "finalizer", "agent": "finalizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "ingest", "to": "planner", "name": "ingest_to_planner" },
                { "from": "planner", "to": "finalizer", "name": "planner_to_final" },
                { "from": "finalizer", "to": "finish", "name": "final_to_finish" }
            ]
        }
    });

    let WorkflowBundle {
        flow,
        agents,
        tools,
    } = load_bundle(config.clone());

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "TestUser",
        "goal": "Record diet",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("ingest".to_string()))?;

    println!("\n========== Test: json_config_driven_flow_executes_chain ==========");
    println!("Initial message: {}", serde_json::to_string_pretty(&json!({
        "user": "TestUser",
        "goal": "Record diet",
        "steps": []
    })).unwrap());

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "json_chain_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: Value = serde_json::from_str(&final_message.content)?;
    let steps = payload["steps"].as_array().expect("steps array");
    
    println!("\n--- Execution Steps ---");
    for (i, step) in steps.iter().enumerate() {
        println!("  Step {}: agent={}, intent={}, driver={}", 
            i + 1,
            step["agent"].as_str().unwrap_or("unknown"),
            step["intent"].as_str().unwrap_or("unknown"),
            step["driver"].as_str().unwrap_or("unknown")
        );
    }
    
    println!("\n--- Final Message ---");
    println!("Role: {}", payload["role"].as_str().unwrap_or("unknown"));
    println!("Model: {}", payload["model"].as_str().unwrap_or("unknown"));
    if let Some(response) = payload.get("response") {
        println!("Response: {}", response.as_str().unwrap_or(""));
    }
    println!("Full message: {}", serde_json::to_string_pretty(&payload).unwrap());
    println!("========================================================\n");
    
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["agent"], "ingest");
    assert_eq!(steps[1]["agent"], "planner");
    assert_eq!(steps[2]["agent"], "finalizer");
    assert_eq!(payload["role"], "Summary Generator");
    assert_eq!(payload["model"], "qwen-max");
    assert!(payload.get("response").is_some(), "Should have Qwen API response");
    let response = payload["response"].as_str().unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(!response.contains("[LLM错误"), "Should not contain LLM error");

    Ok(())
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn json_config_flow_with_decision_node() -> Result<()> {
    let api_key = get_test_api_key();
    let config = json!({
        "agents": [
            {
                "name": "analyzer",
                "driver": "qwen",
                "role": "Requirement Analyst",
                "prompt": "You are a professional requirement analyst, good at analyzing user requirements and extracting key information.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "analyze"
            },
            {
                "name": "route_a",
                "driver": "qwen",
                "role": "Route A Handler",
                "prompt": "You are responsible for handling Route A tasks, focusing on solutions for Type A requirements.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "handle_a"
            },
            {
                "name": "route_b",
                "driver": "qwen",
                "role": "Route B Handler",
                "prompt": "You are responsible for handling Route B tasks, focusing on solutions for Type B requirements.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "handle_b"
            },
            {
                "name": "finalizer",
                "driver": "qwen",
                "role": "Final Handler",
                "prompt": "You are responsible for final processing, organizing results into complete output.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "finalize"
            }
        ],
        "tools": [],
        "flow": {
            "name": "decision_flow",
            "start": "analyzer",
            "nodes": [
                { "kind": "agent", "name": "analyzer", "agent": "analyzer" },
                {
                    "kind": "decision",
                    "name": "route_decision",
                    "policy": "first_match",
                    "branches": [
                        {
                            "target": "route_a",
                            "name": "to_a",
                            "condition": { "type": "state_equals", "key": "route", "value": "a" }
                        },
                        {
                            "target": "route_b",
                            "name": "to_b",
                            "condition": { "type": "state_equals", "key": "route", "value": "b" }
                        }
                    ]
                },
                { "kind": "agent", "name": "route_a", "agent": "route_a" },
                { "kind": "agent", "name": "route_b", "agent": "route_b" },
                { "kind": "agent", "name": "finalizer", "agent": "finalizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "analyzer", "to": "route_decision" },
                { "from": "route_a", "to": "finalizer" },
                { "from": "route_b", "to": "finalizer" },
                { "from": "finalizer", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_bundle(config);

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    ctx.store().set("route", "a".to_string()).await?;

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "TestUser",
        "goal": "Test decision node",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("analyzer".to_string()))?;

    println!("\n========== Test: json_config_flow_with_decision_node ==========");
    println!("Initial message: {}", serde_json::to_string_pretty(&json!({
        "user": "TestUser",
        "goal": "Test decision node",
        "steps": []
    })).unwrap());
    println!("Set state: route = a");

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "decision_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: Value = serde_json::from_str(&final_message.content)?;
    let steps = payload["steps"].as_array().expect("steps array");
    
    println!("\n--- Execution Steps ---");
    for (i, step) in steps.iter().enumerate() {
        println!("  Step {}: agent={}, intent={}, driver={}", 
            i + 1,
            step["agent"].as_str().unwrap_or("unknown"),
            step["intent"].as_str().unwrap_or("unknown"),
            step["driver"].as_str().unwrap_or("unknown")
        );
    }
    
    if let Some(response) = payload.get("response") {
        println!("\n--- Final Response (from Qwen API) ---");
        println!("{}", response.as_str().unwrap_or(""));
    }
    
    println!("\n--- Decision Path Verification ---");
    println!("✓ analyzer executed: {}", steps.iter().any(|s| s["agent"] == "analyzer"));
    println!("✓ route_a executed: {}", steps.iter().any(|s| s["agent"] == "route_a"));
    println!("✓ route_b not executed: {}", !steps.iter().any(|s| s["agent"] == "route_b"));
    println!("✓ finalizer executed: {}", steps.iter().any(|s| s["agent"] == "finalizer"));
    println!("========================================================\n");
    
    assert!(steps.iter().any(|s| s["agent"] == "analyzer"));
    assert!(steps.iter().any(|s| s["agent"] == "route_a"));
    assert!(steps.iter().any(|s| s["agent"] == "finalizer"));
    assert!(!steps.iter().any(|s| s["agent"] == "route_b"));

    Ok(())
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn json_config_flow_with_loop_node() -> Result<()> {
    let api_key = get_test_api_key();
    let config = json!({
        "agents": [
            {
                "name": "init",
                "driver": "qwen",
                "role": "initializer",
                "prompt": "Initialize loop",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "init"
            },
            {
                "name": "processor",
                "driver": "qwen",
                "role": "processor",
                "prompt": "Process loop task",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "process"
            },
            {
                "name": "finalizer",
                "driver": "qwen",
                "role": "finalizer",
                "prompt": "Complete processing",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "finalize"
            }
        ],
        "tools": [],
        "flow": {
            "name": "loop_flow",
            "start": "init",
            "nodes": [
                { "kind": "agent", "name": "init", "agent": "init" },
                {
                    "kind": "loop",
                    "name": "process_loop",
                    "entry": "processor",
                    "condition": {
                        "state_equals": { "key": "continue", "value": "true" }
                    },
                    "max_iterations": 1,
                    "exit": "finalizer"
                },
                { "kind": "agent", "name": "processor", "agent": "processor" },
                { "kind": "agent", "name": "finalizer", "agent": "finalizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "init", "to": "process_loop" },
                { "from": "processor", "to": "process_loop" },
                { "from": "finalizer", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_bundle(config);

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    ctx.store().set("continue", "true".to_string()).await?;
    ctx.store().set("iteration", "0".to_string()).await?;

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "TestUser",
        "goal": "Test loop node",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("init".to_string()))?;

    println!("\n========== Test: json_config_flow_with_loop_node ==========");
    println!("Initial message: {}", serde_json::to_string_pretty(&json!({
        "user": "TestUser",
        "goal": "Test loop node",
        "steps": []
    })).unwrap());
    println!("Set state: continue = true, iteration = 0");
    println!("Loop config: max_iterations = 1");

    let result = executor.start(Arc::clone(&ctx), initial_message).await;

    match result {
        Ok(exec_result) => {
            println!("\n--- Loop Node Execution Result (Normal Completion) ---");
            println!("Flow name: {}", exec_result.flow_name);
            println!("Last node: {}", exec_result.last_node);
            if let Some(final_message) = exec_result.last_message {
                let payload: Value = serde_json::from_str(&final_message.content)?;
                if let Some(steps) = payload["steps"].as_array() {
                    println!("\n--- Execution Steps ({} total) ---", steps.len());
                    for (i, step) in steps.iter().enumerate() {
                        println!("  Step {}: agent={}, intent={}", 
                            i + 1,
                            step["agent"].as_str().unwrap_or("unknown"),
                            step["intent"].as_str().unwrap_or("unknown")
                        );
                    }
                }
            }
            assert_eq!(exec_result.flow_name, "loop_flow");
        }
        Err(e) => {
            println!("\n--- Loop Node Execution Result (Max Iterations Reached) ---");
            let error_msg = format!("{}", e);
            println!("Error message: {}", error_msg);
            println!("This is expected behavior: loop node throws error when max_iterations is reached");
            assert!(
                error_msg.contains("exceeded maximum iterations") || error_msg.contains("loop"),
                "Error should be about loop node or max iterations: {}",
                error_msg
            );
        }
    }
    println!("========================================================\n");

    Ok(())
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn json_config_flow_with_conditional_transitions() -> Result<()> {
    let api_key = get_test_api_key();
    let config = json!({
        "agents": [
            {
                "name": "checker",
                "driver": "qwen",
                "role": "checker",
                "prompt": "Check condition",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "check"
            },
            {
                "name": "handler_a",
                "driver": "qwen",
                "role": "handler_a",
                "prompt": "Handle A",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "handle_a"
            },
            {
                "name": "handler_b",
                "driver": "qwen",
                "role": "handler_b",
                "prompt": "Handle B",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "handle_b"
            },
            {
                "name": "finalizer",
                "driver": "qwen",
                "role": "finalizer",
                "prompt": "Final processing",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "finalize"
            }
        ],
        "tools": [],
        "flow": {
            "name": "conditional_flow",
            "start": "checker",
            "nodes": [
                { "kind": "agent", "name": "checker", "agent": "checker" },
                { "kind": "agent", "name": "handler_a", "agent": "handler_a" },
                { "kind": "agent", "name": "handler_b", "agent": "handler_b" },
                { "kind": "agent", "name": "finalizer", "agent": "finalizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                {
                    "from": "checker",
                    "to": "handler_a",
                    "name": "to_a",
                    "condition": { "type": "state_equals", "key": "path", "value": "a" }
                },
                {
                    "from": "checker",
                    "to": "handler_b",
                    "name": "to_b",
                    "condition": { "type": "state_equals", "key": "path", "value": "b" }
                },
                { "from": "handler_a", "to": "finalizer" },
                { "from": "handler_b", "to": "finalizer" },
                { "from": "finalizer", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_bundle(config);

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    ctx.store().set("path", "a".to_string()).await?;

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "TestUser",
        "goal": "Test conditional transitions",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("checker".to_string()))?;

    println!("\n========== Test: json_config_flow_with_conditional_transitions ==========");
    println!("Initial message: {}", serde_json::to_string_pretty(&json!({
        "user": "TestUser",
        "goal": "Test conditional transitions",
        "steps": []
    })).unwrap());
    println!("Set state: path = a");

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "conditional_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: Value = serde_json::from_str(&final_message.content)?;
    let steps = payload["steps"].as_array().expect("steps array");
    
    println!("\n--- Execution Steps ---");
    for (i, step) in steps.iter().enumerate() {
        println!("  Step {}: agent={}, intent={}, driver={}", 
            i + 1,
            step["agent"].as_str().unwrap_or("unknown"),
            step["intent"].as_str().unwrap_or("unknown"),
            step["driver"].as_str().unwrap_or("unknown")
        );
    }
    
    if let Some(response) = payload.get("response") {
        println!("\n--- Final Response (from Qwen API) ---");
        println!("{}", response.as_str().unwrap_or(""));
    }
    
    println!("\n--- Conditional Transition Path Verification ---");
    println!("✓ checker executed: {}", steps.iter().any(|s| s["agent"] == "checker"));
    println!("✓ handler_a executed: {}", steps.iter().any(|s| s["agent"] == "handler_a"));
    println!("✓ handler_b not executed: {}", !steps.iter().any(|s| s["agent"] == "handler_b"));
    println!("✓ finalizer executed: {}", steps.iter().any(|s| s["agent"] == "finalizer"));
    println!("========================================================\n");
    
    assert!(steps.iter().any(|s| s["agent"] == "checker"));
    assert!(steps.iter().any(|s| s["agent"] == "handler_a"));
    assert!(steps.iter().any(|s| s["agent"] == "finalizer"));
    assert!(!steps.iter().any(|s| s["agent"] == "handler_b"));

    Ok(())
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn json_config_flow_with_join_node() -> Result<()> {
    let api_key = get_test_api_key();
    let config = json!({
        "agents": [
            {
                "name": "splitter",
                "driver": "qwen",
                "role": "splitter",
                "prompt": "Split processing",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "split"
            },
            {
                "name": "worker_a",
                "driver": "qwen",
                "role": "worker_a",
                "prompt": "Worker node A",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "work_a"
            },
            {
                "name": "worker_b",
                "driver": "qwen",
                "role": "worker_b",
                "prompt": "Worker node B",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "work_b"
            },
            {
                "name": "merger",
                "driver": "qwen",
                "role": "merger",
                "prompt": "Merge results",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "merge"
            },
            {
                "name": "finalizer",
                "driver": "qwen",
                "role": "finalizer",
                "prompt": "Final processing",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "finalize"
            }
        ],
        "tools": [],
        "flow": {
            "name": "join_flow",
            "start": "splitter",
            "nodes": [
                { "kind": "agent", "name": "splitter", "agent": "splitter" },
                { "kind": "agent", "name": "worker_a", "agent": "worker_a" },
                { "kind": "agent", "name": "worker_b", "agent": "worker_b" },
                {
                    "kind": "join",
                    "name": "merge_join",
                    "strategy": "all",
                    "inbound": ["worker_a", "worker_b"]
                },
                { "kind": "agent", "name": "merger", "agent": "merger" },
                { "kind": "agent", "name": "finalizer", "agent": "finalizer" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "splitter", "to": "worker_a" },
                { "from": "splitter", "to": "worker_b" },
                { "from": "merge_join", "to": "merger" },
                { "from": "merger", "to": "finalizer" },
                { "from": "finalizer", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_bundle(config);

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "TestUser",
        "goal": "Test Join node",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("splitter".to_string()))?;

    println!("\n========== Test: json_config_flow_with_join_node ==========");
    println!("Initial message: {}", serde_json::to_string_pretty(&json!({
        "user": "TestUser",
        "goal": "Test Join node",
        "steps": []
    })).unwrap());

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "join_flow");
    
    println!("\n--- Join Node Execution Result ---");
    println!("Flow name: {}", result.flow_name);
    println!("Last node: {}", result.last_node);
    
    if let Some(final_message) = result.last_message {
        let payload: Value = serde_json::from_str(&final_message.content)?;
        if let Some(steps) = payload["steps"].as_array() {
            println!("\n--- Execution Steps ({} total) ---", steps.len());
            for (i, step) in steps.iter().enumerate() {
                println!("  Step {}: agent={}, intent={}, driver={}", 
                    i + 1,
                    step["agent"].as_str().unwrap_or("unknown"),
                    step["intent"].as_str().unwrap_or("unknown"),
                    step["driver"].as_str().unwrap_or("unknown")
                );
            }
            assert!(!steps.is_empty(), "Should have at least some nodes executed");
            
            if let Some(response) = payload.get("response") {
                println!("\n--- Final Response (from Qwen API) ---");
                println!("{}", response.as_str().unwrap_or(""));
            }
        }
        println!("\nFull message: {}", serde_json::to_string_pretty(&payload).unwrap());
    }
    println!("========================================================\n");

    Ok(())
}

#[tokio::test]
#[cfg(feature = "openai-client")]
async fn json_config_multi_agent_conversation() -> Result<()> {
    let api_key = get_test_api_key();
    
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

    let WorkflowBundle { flow, agents, tools } = load_bundle(config);

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "ProductOwner",
        "goal": "We need to develop an intelligent customer service system that can automatically answer user questions, support multi-turn conversations, and integrate with multiple business systems. Please discuss and provide a complete solution.",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("product_manager".to_string()))?;

    println!("\n========== Test: Multi-Role Multi-Agent Conversation ==========");
    println!("Scenario: Intelligent Customer Service System Development Requirement Discussion");
    println!("\n--- User Requirement ---");
    println!("User: ProductOwner");
    println!("Requirement: We need to develop an intelligent customer service system that can automatically answer user questions, support multi-turn conversations, and integrate with multiple business systems. Please discuss and provide a complete solution.");
    println!("\n--- Conversation Flow ---");
    println!("1. Product Manager -> Analyze requirements, propose product solution");
    println!("2. Technical Expert -> Evaluate technical feasibility, propose technical architecture");
    println!("3. Designer -> Propose UI/UX design solution");
    println!("4. Project Manager -> Create project implementation plan");
    println!("5. Summarizer -> Generate final summary report");
    println!("\n--- Starting Execution ---\n");

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "multi_agent_conversation_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: Value = serde_json::from_str(&final_message.content)?;
    let steps = payload["steps"].as_array().expect("steps array");

    println!("\n--- Conversation Execution Record ---");
    println!("Total conversation rounds: {}", steps.len());
    println!("\n--- Detailed Conversation Content ---\n");
    
    for (i, step) in steps.iter().enumerate() {
        let agent_name = step["agent"].as_str().unwrap_or("unknown");
        let role = step.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let intent = step["intent"].as_str().unwrap_or("unknown");
        
        println!("[Round {}] {} ({})", i + 1, role, agent_name);
        println!("Intent: {}", intent);
        
        if let Some(response) = step.get("response") {
            let response_text = response.as_str().unwrap_or("");
            if !response_text.is_empty() {
                println!("Response: {}", response_text);
            }
        }
        println!();
    }

    if let Some(response) = payload.get("response") {
        println!("--- Final Summary Report (from Summarizer) ---");
        println!("{}\n", response.as_str().unwrap_or(""));
    }

    println!("--- Conversation Verification ---");
    println!("✓ Product Manager participated: {}", steps.iter().any(|s| s["agent"] == "product_manager"));
    println!("✓ Technical Expert participated: {}", steps.iter().any(|s| s["agent"] == "tech_expert"));
    println!("✓ Designer participated: {}", steps.iter().any(|s| s["agent"] == "designer"));
    println!("✓ Project Manager participated: {}", steps.iter().any(|s| s["agent"] == "project_manager"));
    println!("✓ Summarizer participated: {}", steps.iter().any(|s| s["agent"] == "summarizer"));
    println!("✓ Conversation rounds: {}", steps.len());
    
    assert!(steps.iter().any(|s| s["agent"] == "product_manager"), "Product Manager should participate");
    assert!(steps.iter().any(|s| s["agent"] == "tech_expert"), "Technical Expert should participate");
    assert!(steps.iter().any(|s| s["agent"] == "designer"), "Designer should participate");
    assert!(steps.iter().any(|s| s["agent"] == "project_manager"), "Project Manager should participate");
    assert!(steps.iter().any(|s| s["agent"] == "summarizer"), "Summarizer should participate");
    assert_eq!(steps.len(), 5, "Should have 5 conversation rounds");

    assert!(payload.get("response").is_some(), "Should have Qwen API response");
    let response = payload["response"].as_str().unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(!response.contains("[LLM错误"), "Should not contain LLM error");

    println!("\n--- Full Conversation Record (JSON) ---");
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    println!("\n========================================================\n");

    Ok(())
}
