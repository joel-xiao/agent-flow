use std::sync::Arc;

use agentflow::state::MemoryStore;
use agentflow::{
    FlowContext, FlowExecutor, MessageRole, StructuredMessage, WorkflowBundle,
    load_workflow_from_value,
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let image_path = std::env::args().nth(1).ok_or_else(|| anyhow::anyhow!("Missing image path parameter"))?;
    
    if !std::path::Path::new(&image_path).exists() {
        return Err(anyhow::anyhow!("Image file '{}' does not exist", image_path));
    }

    let image_base64 = general_purpose::STANDARD.encode(std::fs::read(&image_path)?);
    let api_key = std::env::var("QWEN_API_KEY")?;

    println!("Food Vision Analysis Multi-Agent Example");
    println!("{}", "=".repeat(60));
    println!("Analyzing image: {}", image_path);
    println!("\nStarting multi-expert collaborative analysis...\n");

    let config = json!({
        "agents": [
            {
                "name": "food_identifier",
                "driver": "qwen",
                "role": "Food Identification Expert",
                "prompt": "You are an experienced food identification expert, good at identifying various foods from images. Please carefully analyze this food image and identify all visible food types and names. Describe all the foods you see in natural language, including their appearance, color, and possible ingredients.",
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
            "name": "food_vision_analysis_flow",
            "start": "food_identifier",
            "nodes": [
                { "kind": "agent", "name": "food_identifier", "agent": "food_identifier" },
                { "kind": "agent", "name": "portion_estimator", "agent": "portion_estimator" },
                { "kind": "agent", "name": "calorie_calculator", "agent": "calorie_calculator" },
                { "kind": "agent", "name": "health_advisor", "agent": "health_advisor" },
                { "kind": "agent", "name": "fitness_coach", "agent": "fitness_coach" },
                { "kind": "join", "name": "expert_join", "strategy": "all" },
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

    let bundle: WorkflowBundle = load_workflow_from_value(&config)?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    let initial_message = StructuredMessage::new(json!({
        "user": "User",
        "goal": "Analyze this food image, identify foods, calculate calories, and provide health advice",
        "image_path": image_path,
        "image_base64": format!("data:image/jpeg;base64,{}", image_base64),
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("food_identifier".to_string()))?;

    let result = executor.start(ctx, initial_message).await?;

    println!("\n{}", "=".repeat(60));
    println!("Analysis completed!");
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
            println!("\nFinal diet planning recommendation:");
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
