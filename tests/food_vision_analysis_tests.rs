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
async fn food_vision_multi_agent_conversation() -> Result<()> {
    let api_key = get_test_api_key();
    
    let image_path = "./tests/test_food.jpg";
    let image_bytes = std::fs::read(image_path)?;
    use base64::{Engine as _, engine::general_purpose};
    let image_base64 = general_purpose::STANDARD.encode(&image_bytes);

    let config = json!({
        "agents": [
            {
                "name": "food_identifier",
                "driver": "qwen",
                "role": "Food Identification Expert",
                "prompt": "You are an experienced food identification expert, good at identifying various foods from images. Please carefully analyze this food image and identify all visible food types and names. Describe all the foods you see in natural language, including their appearance, color, and possible ingredients. If there is a taco in the image, please clearly indicate it.",
                "model": "qwen3-vl-plus",
                "api_key": api_key,
                "intent": "identify"
            },
            {
                "name": "portion_estimator",
                "driver": "qwen",
                "role": "Portion Estimation Expert",
                "prompt": "You are a professional portion estimation expert, good at estimating the actual weight of food based on images. Based on the food identification expert's analysis, carefully examine the image and estimate the weight (in grams) of each food item. Explain your estimation basis, such as food size, thickness, and common portions.",
                "model": "qwen3-vl-plus",
                "api_key": api_key,
                "intent": "estimate"
            },
            {
                "name": "calorie_calculator",
                "driver": "qwen",
                "role": "Nutrition Calculation Expert",
                "prompt": "You are a professional nutrition calculation expert, good at calculating calories and nutritional content of food. Based on the food identification and portion estimation experts' analysis, calculate the calories for each food item. Provide detailed calculation process, including nutritional content (protein, fat, carbohydrates, etc.) for each food.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "calculate"
            },
            {
                "name": "health_advisor",
                "driver": "qwen",
                "role": "Health Advisor",
                "prompt": "You are a senior health advisor, good at providing health advice based on nutritional data. Based on the nutrition calculation expert's analysis results, evaluate the healthiness of this meal, including: 1. Whether the total calories of this meal are suitable for the current meal; 2. Nutritional balance analysis (ratio of protein, fat, carbohydrates); 3. Potential health impacts; 4. Specific health recommendations. Please answer in professional but understandable language.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "health_advice"
            },
            {
                "name": "fitness_coach",
                "driver": "qwen",
                "role": "Fitness Coach",
                "prompt": "You are a professional fitness coach, good at creating exercise plans based on calorie data. Based on the calorie data provided by the nutrition calculation expert, provide exercise recommendations, including: 1. What exercises are needed to burn these calories; 2. Exercise duration and intensity recommendations; 3. Suitable exercise types. Please provide specific and actionable recommendations.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "fitness_advice"
            },
            {
                "name": "diet_planner",
                "driver": "qwen",
                "role": "Diet Planner",
                "prompt": "You are a professional diet planner, good at integrating multi-faceted nutritional and health information to create complete diet plans. Based on all previous experts' analysis (food identification, portion estimation, nutrition calculation, health advice, exercise advice), create a complete diet planning recommendation for the user, including: 1. Overall evaluation of this meal; 2. How to pair with other foods to achieve nutritional balance; 3. Three-meal pairing recommendations; 4. Long-term diet planning recommendations. Please present in a clear and structured manner.",
                "model": "qwen-max",
                "api_key": api_key,
                "intent": "diet_planning"
            }
        ],
        "tools": [],
        "flow": {
            "name": "food_multi_agent_conversation_flow",
            "start": "food_identifier",
            "nodes": [
                { "kind": "agent", "name": "food_identifier", "agent": "food_identifier" },
                { "kind": "agent", "name": "portion_estimator", "agent": "portion_estimator" },
                { "kind": "agent", "name": "calorie_calculator", "agent": "calorie_calculator" },
                { "kind": "agent", "name": "health_advisor", "agent": "health_advisor" },
                { "kind": "agent", "name": "fitness_coach", "agent": "fitness_coach" },
                {
                    "kind": "join",
                    "name": "expert_join",
                    "strategy": "all",
                    "inbound": ["health_advisor", "fitness_coach"]
                },
                { "kind": "agent", "name": "diet_planner", "agent": "diet_planner" },
                { "kind": "terminal", "name": "finish" }
            ],
            "transitions": [
                { "from": "food_identifier", "to": "portion_estimator" },
                { "from": "portion_estimator", "to": "calorie_calculator" },
                { "from": "calorie_calculator", "to": "health_advisor" },
                { "from": "calorie_calculator", "to": "fitness_coach" },
                { "from": "health_advisor", "to": "expert_join" },
                { "from": "fitness_coach", "to": "expert_join" },
                { "from": "expert_join", "to": "diet_planner" },
                { "from": "diet_planner", "to": "finish" }
            ]
        }
    });

    let WorkflowBundle { flow, agents, tools } = load_workflow_from_value(&config)?;

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, tools);

    println!("\n========== Test: Multi-Role Multi-Agent Food Analysis Conversation ==========");
    println!("Scenario: Food Image Analysis - Multi-Expert Collaborative Discussion");
    println!("\n--- Image Information ---");
    println!("Image file: {}", image_path);
    println!("Image size: {} bytes", image_bytes.len());
    println!("\n--- Participating Roles ---");
    println!("1. Food Identification Expert - Identify foods in the image");
    println!("2. Portion Estimation Expert - Estimate food portions");
    println!("3. Nutrition Calculation Expert - Calculate calories and nutritional content");
    println!("4. Health Advisor - Provide health advice based on nutritional data");
    println!("5. Fitness Coach - Provide exercise advice based on calorie data");
    println!("6. Diet Planner - Integrate all information and provide complete diet planning");
    println!("\n--- Conversation Flow ---");
    println!("User uploads image -> Food identification -> Portion estimation -> Nutrition calculation -> [Health Advisor + Fitness Coach] -> Diet Planner -> Complete");
    println!("\n--- Starting Execution ---\n");

    let initial_message = StructuredMessage::new(json!({
        "user": "User",
        "goal": "Please analyze this food image, identify foods, estimate portions, calculate calories and provide nutritional advice. The image contains a taco, please identify it carefully.",
        "image_base64": image_base64,
        "image_path": image_path,
        "raw": "Please analyze this food image, identify foods, estimate portions, calculate calories and provide nutritional advice",
        "steps": [],
        "conversation_context": {
            "topic": "Food Nutrition Analysis",
            "image_analysis": true
        }
    }))
    .into_agent_message(MessageRole::User, "client", Some("food_identifier".to_string()))?;

    let result = executor.start(Arc::clone(&ctx), initial_message).await?;

    assert_eq!(result.flow_name, "food_multi_agent_conversation_flow");
    assert_eq!(result.last_node, "finish");

    let final_message = result.last_message.expect("final message exists");
    let payload: serde_json::Value = serde_json::from_str(&final_message.content)?;
    let steps = payload["steps"].as_array().expect("steps array");

    println!("\n--- Conversation Execution Record ---");
    println!("Total conversation rounds: {}", steps.len());
    println!("\n--- Detailed Conversation Content ---\n");
    
    for (i, step) in steps.iter().enumerate() {
        let agent_name = step["agent"].as_str().unwrap_or("unknown");
        let role = step.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let intent = step["intent"].as_str().unwrap_or("unknown");
        let driver = step["driver"].as_str().unwrap_or("unknown");
        
        println!("[Round {}] {} ({})", i + 1, role, agent_name);
        println!("  Intent: {}", intent);
        println!("  Driver: {}", driver);
        
        if let Some(response) = step.get("response") {
            let response_text = response.as_str().unwrap_or("");
            if !response_text.is_empty() {
                let preview = if response_text.len() > 200 {
                    format!("{}...", &response_text[..200])
                } else {
                    response_text.to_string()
                };
                println!("  Response: {}", preview);
            }
        }
        println!();
    }

    if let Some(response) = payload.get("response") {
        println!("--- Final Diet Planning Report (from Diet Planner) ---");
        println!("{}\n", response.as_str().unwrap_or(""));
    }

    println!("--- Conversation Verification ---");
    println!("✓ Food Identification Expert participated: {}", steps.iter().any(|s| s["agent"] == "food_identifier"));
    println!("✓ Portion Estimation Expert participated: {}", steps.iter().any(|s| s["agent"] == "portion_estimator"));
    println!("✓ Nutrition Calculation Expert participated: {}", steps.iter().any(|s| s["agent"] == "calorie_calculator"));
    println!("✓ Health Advisor participated: {}", steps.iter().any(|s| s["agent"] == "health_advisor"));
    println!("✓ Fitness Coach participated: {}", steps.iter().any(|s| s["agent"] == "fitness_coach"));
    println!("✓ Diet Planner participated: {}", steps.iter().any(|s| s["agent"] == "diet_planner"));
    println!("✓ Conversation rounds: {}", steps.len());
    
    assert!(steps.iter().any(|s| s["agent"] == "food_identifier"), "Food Identification Expert should participate");
    assert!(steps.iter().any(|s| s["agent"] == "portion_estimator"), "Portion Estimation Expert should participate");
    assert!(steps.iter().any(|s| s["agent"] == "calorie_calculator"), "Nutrition Calculation Expert should participate");
    let has_health_or_fitness = steps.iter().any(|s| s["agent"] == "health_advisor") 
        || steps.iter().any(|s| s["agent"] == "fitness_coach");
    assert!(has_health_or_fitness, "Health Advisor or Fitness Coach should participate");
    assert!(steps.iter().any(|s| s["agent"] == "diet_planner"), "Diet Planner should participate");
    
    assert!(payload.get("response").is_some(), "Should have Qwen API response");
    let response = payload["response"].as_str().unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(!response.contains("[LLM错误"), "Should not contain LLM error");

    println!("\n--- Full Conversation Record (JSON) ---");
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    println!("\n========================================================\n");

    Ok(())
}
