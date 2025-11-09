#![cfg(feature = "openai-client")]

use std::collections::HashMap;
use std::sync::Arc;

use agentflow::agent::{Agent, AgentAction, AgentContext, AgentMessage};
use agentflow::error::AgentFlowError;
use agentflow::state::{FlowContext, MemoryStore};
use agentflow::tools::Tool;
use agentflow::{
    AgentRegistry, FlowBuilder, FlowExecutor, MessageRole, ToolInvocation, ToolRegistry,
    register_agent,
};
use anyhow::anyhow;
use async_trait::async_trait;
use parking_lot::Mutex;
use reqwest::Client;
use serde_json::{Value, json};

fn new_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("diet-msg-{}-{}", now.as_secs(), now.subsec_nanos())
}

/// 视觉识别代理：通过 Qwen 视觉模型识别食物
struct QwenVisionAgent {
    next: String,
    conversation: Arc<Mutex<Vec<String>>>,
}

impl QwenVisionAgent {
    fn new(next: impl Into<String>, conversation: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            next: next.into(),
            conversation,
        }
    }
}

#[async_trait]
impl Agent for QwenVisionAgent {
    fn name(&self) -> &'static str {
        "vision_agent"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let payload: Value =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let image_url = payload["image_url"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow!("missing image_url")))?;
        let prompt = payload["prompt"].as_str().unwrap_or(
            "你是一名视觉助手，请识别图片中的食物，严格返回 JSON：{\"foods\": [\"食物名称\"...], \"conversation\": \"一句话描述\"}，仅返回 JSON。",
        );

        self.conversation
            .lock()
            .push(format!("[User] 图像地址: {}\n提示: {}", image_url, prompt));

        let vision_output = ctx
            .runtime
            .call_tool(
                "qwen.vision",
                ToolInvocation::new(
                    "qwen.vision",
                    json!({
                        "image_url": image_url,
                        "prompt": prompt
                    }),
                ),
            )
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        self.conversation
            .lock()
            .push(format!("[Vision] {}", vision_output.content));

        let mut payload: Value = serde_json::from_str(&vision_output.content)
            .map_err(|e| AgentFlowError::Other(e.into()))?;
        payload["image_url"] = Value::String(image_url.to_string());

        let foods_payload: Value = serde_json::from_str(&vision_output.content)
            .map_err(|e| AgentFlowError::Other(e.into()))?;
        let foods = foods_payload["foods"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if foods.is_empty() {
            return Err(AgentFlowError::Other(anyhow!(
                "vision model did not return foods"
            )));
        }

        let next_message = AgentMessage {
            id: new_message_id(),
            role: MessageRole::Agent,
            from: self.name().to_string(),
            to: Some(self.next.clone()),
            content: payload.to_string(),
            metadata: None,
        };

        Ok(AgentAction::Next {
            target: self.next.clone(),
            message: next_message,
        })
    }
}

/// 分量估算代理：再次调用 Qwen 视觉模型估计每种食物的克重
struct QwenPortionAgent {
    next: String,
    conversation: Arc<Mutex<Vec<String>>>,
}

impl QwenPortionAgent {
    fn new(next: impl Into<String>, conversation: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            next: next.into(),
            conversation,
        }
    }
}

#[async_trait]
impl Agent for QwenPortionAgent {
    fn name(&self) -> &'static str {
        "portion_agent"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let vision_data: Value =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let foods = vision_data["foods"].as_array().cloned().unwrap_or_default();
        let image_url = vision_data["image_url"].as_str().ok_or_else(|| {
            AgentFlowError::Other(anyhow!("vision data missing image_url for portion agent"))
        })?;
        let food_list = foods.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>();

        let portion_prompt = format!(
            "请基于图像估计以下食物的克重，返回 JSON 数组，每项 {{\"name\": 食物名称, \"grams\": 整数克重}}，仅输出 JSON。食物列表：{}",
            food_list.join("、")
        );

        let invocation = ToolInvocation::new(
            "qwen.vision",
            json!({
                "image_url": image_url,
                "prompt": portion_prompt
            }),
        );
        let portion_result = ctx
            .runtime
            .call_tool("qwen.vision", invocation)
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        self.conversation
            .lock()
            .push(format!("[Portion] {}", portion_result.content));

        let items: Value = match serde_json::from_str(&portion_result.content) {
            Ok(v) => v,
            Err(_) => {
                let content = portion_result.content.trim();
                let start = content.find('[');
                let end = content.rfind(']');
                if let (Some(s), Some(e)) = (start, end) {
                    let slice = &content[s..=e];
                    serde_json::from_str(slice).map_err(|e| AgentFlowError::Other(e.into()))?
                } else {
                    return Err(AgentFlowError::Other(anyhow!(
                        "portion model did not return valid JSON: {}",
                        content
                    )));
                }
            }
        };

        let mut enriched = vision_data.clone();
        enriched["items"] = items;

        let next_message = AgentMessage {
            id: new_message_id(),
            role: MessageRole::Agent,
            from: self.name().to_string(),
            to: Some(self.next.clone()),
            content: enriched.to_string(),
            metadata: None,
        };

        Ok(AgentAction::Next {
            target: self.next.clone(),
            message: next_message,
        })
    }
}

/// 计算食品总热量的代理
struct CalorieCalculatorAgent {
    next: String,
    known_calories: HashMap<String, u32>,
    conversation: Arc<Mutex<Vec<String>>>,
}

impl CalorieCalculatorAgent {
    fn new(next: impl Into<String>, conversation: Arc<Mutex<Vec<String>>>) -> Self {
        let mut known = HashMap::new();
        known.insert("沙拉".into(), 150);
        known.insert("鸡胸肉".into(), 200);
        known.insert("牛油果".into(), 160);
        known.insert("酸奶".into(), 120);
        known.insert("蛋白奶昔".into(), 180);
        known.insert("水果".into(), 90);
        Self {
            next: next.into(),
            known_calories: known,
            conversation,
        }
    }
}

#[async_trait]
impl Agent for CalorieCalculatorAgent {
    fn name(&self) -> &'static str {
        "calorie_calculator"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let data: Value =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let items = data["items"].as_array().cloned().unwrap_or_default();
        let mut detail = Vec::new();
        let mut steps = Vec::new();
        let mut total = 0;
        for item in items {
            if let (Some(name), Some(grams)) = (item["name"].as_str(), item["grams"].as_f64()) {
                let per100 = self.known_calories.get(name).cloned().unwrap_or(150);
                let calories = (per100 as f64) * grams / 100.0;
                total += calories.round() as u32;
                detail.push(format!("{}:{} kcal", name, calories.round() as u32));
                steps.push(format!("{}≈{}g", name, grams.round() as u32));
            }
        }

        let transcript = format!(
            "[CalorieCalc] 估算总热量 {} kcal ({})",
            total,
            if steps.is_empty() {
                "无法识别具体食物".to_string()
            } else {
                steps.join("，")
            }
        );
        self.conversation.lock().push(transcript.clone());

        let summary = json!({
            "total": total,
            "detail": detail,
            "conversation": transcript,
        });

        ctx.flow_ctx
            .store()
            .set("diet.total_calories", total.to_string())
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        let next_message = AgentMessage {
            id: new_message_id(),
            role: MessageRole::Agent,
            from: self.name().to_string(),
            to: Some(self.next.clone()),
            content: summary.to_string(),
            metadata: None,
        };

        Ok(AgentAction::Next {
            target: self.next.clone(),
            message: next_message,
        })
    }
}

/// 审查角色：结合视觉结果和热量估算调用千问文本模型给出建议
struct QwenReviewerAgent {
    max_calories: u32,
    output: Arc<Mutex<Option<String>>>,
    conversation: Arc<Mutex<Vec<String>>>,
}

impl QwenReviewerAgent {
    fn new(
        max_calories: u32,
        output: Arc<Mutex<Option<String>>>,
        conversation: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            max_calories,
            output,
            conversation,
        }
    }
}

#[async_trait]
impl Agent for QwenReviewerAgent {
    fn name(&self) -> &'static str {
        "reviewer"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let data: Value =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let total = data["total"].as_u64().unwrap_or_default() as u32;
        let detail = data["detail"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        let description = data["conversation"].as_str().unwrap_or("无描述");

        let convo_snapshot = self.conversation.lock().clone();
        let prompt = format!(
            "你是一名专业营养师，正在协助用户进行减脂饮食评估。\n目标热量阈值：{} kcal\n视觉识别描述：{}\n估算食物热量：{}。\n总热量：{} kcal。\n此前的对话记录：\n{}\n请用中文给出简洁总结，说明是否适合减脂、理由以及三条可执行建议。",
            self.max_calories,
            description,
            if detail.is_empty() {
                "未知".to_string()
            } else {
                detail.join("，")
            },
            total,
            if convo_snapshot.is_empty() {
                "暂无".to_string()
            } else {
                convo_snapshot.join("\n")
            }
        );

        let invocation =
            ToolInvocation::new("qwen.text", json!({ "prompt": prompt, "max_tokens": 512 }));
        let mut tool_message = ctx
            .runtime
            .call_tool("qwen.text", invocation)
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        self.conversation
            .lock()
            .push(format!("[Qwen] {}", tool_message.content));
        *self.output.lock() = Some(tool_message.content.clone());

        ctx.flow_ctx
            .store()
            .set("diet.review_result", tool_message.content.clone())
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        tool_message.role = MessageRole::Agent;
        tool_message.from = self.name().to_string();

        Ok(AgentAction::Finish {
            message: Some(tool_message),
        })
    }
}

#[tokio::test]
async fn diet_flow_recommendation() -> anyhow::Result<()> {
    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let mut agents = AgentRegistry::new();
    let review_output = Arc::new(Mutex::new(None));
    let conversation = Arc::new(Mutex::new(Vec::new()));

    register_agent(
        "vision_agent",
        Arc::new(QwenVisionAgent::new(
            "portion_agent",
            Arc::clone(&conversation),
        )),
        &mut agents,
    );
    register_agent(
        "portion_agent",
        Arc::new(QwenPortionAgent::new(
            "calorie_calculator",
            Arc::clone(&conversation),
        )),
        &mut agents,
    );
    register_agent(
        "calorie_calculator",
        Arc::new(CalorieCalculatorAgent::new(
            "reviewer",
            Arc::clone(&conversation),
        )),
        &mut agents,
    );
    register_agent(
        "reviewer",
        Arc::new(QwenReviewerAgent::new(
            500,
            Arc::clone(&review_output),
            Arc::clone(&conversation),
        )),
        &mut agents,
    );

    let mut builder = FlowBuilder::new("diet");
    builder
        .add_agent_node("vision_agent", "vision_agent")
        .add_agent_node("portion_agent", "portion_agent")
        .add_agent_node("calorie_calculator", "calorie_calculator")
        .add_agent_node("reviewer", "reviewer")
        .add_terminal_node("done")
        .set_start("vision_agent")
        .connect("vision_agent", "portion_agent")
        .connect("portion_agent", "calorie_calculator")
        .connect("calorie_calculator", "reviewer")
        .connect("reviewer", "done");

    let flow = builder.build();

    let mut tools = ToolRegistry::new();
    let api_key = std::env::var("QWEN_API_KEY")
        .or_else(|_| std::env::var("DASHSCOPE_API_KEY"))
        .map_err(|_| anyhow!("Qwen API key not provided"))?;
    let base_url = std::env::var("QWEN_BASE_URL").unwrap_or_else(|_| {
        "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string()
    });

    tools.register(Arc::new(QwenVisionTool::new(
        base_url.clone(),
        api_key.clone(),
    )));
    tools.register(Arc::new(QwenTextTool::new(
        base_url.clone(),
        api_key.clone(),
    )));

    let executor = FlowExecutor::new(flow, agents, tools);

    let input = json!({
        "image_url": "https://img0.baidu.com/it/u=386398022,1220280026&fm=253&fmt=auto&app=120&f=JPEG?w=1200&h=800",
        "prompt": "请识别图片中的主要食物，严格输出 JSON 结构 {\"foods\": [\"食物名称\"...], \"conversation\": \"简短描述\"}"
    })
    .to_string();
    let result = executor.start(ctx, AgentMessage::user(input)).await?;

    assert_eq!(result.flow_name, "diet");
    assert_eq!(result.last_node, "reviewer");
    let final_message = result
        .last_message
        .expect("reviewer should return final message");
    assert!(
        final_message.content.contains("建议")
            || final_message.content.contains("减脂")
            || final_message.content.contains("热量")
    );

    let stored = review_output
        .lock()
        .clone()
        .expect("verdict should be stored");
    assert!(stored.contains("建议") || stored.contains("推荐"));

    let stored_total = ctx_store
        .get("diet.total_calories")
        .await?
        .expect("total calories stored");
    assert!(!stored_total.is_empty());

    let transcript = conversation.lock().clone();
    println!("\n[DietFlow Transcript]");
    for line in &transcript {
        println!("  {}", line);
    }
    assert!(
        transcript.iter().any(|line| line.contains("[Vision]")),
        "transcript should contain vision agent line: {:?}",
        transcript
    );
    assert!(
        transcript.iter().any(|line| line.contains("[Portion]")),
        "transcript should contain portion agent line: {:?}",
        transcript
    );
    assert!(
        transcript.iter().any(|line| line.contains("[CalorieCalc]")),
        "transcript should contain calorie calculator line: {:?}",
        transcript
    );
    assert!(
        transcript.iter().any(|line| line.contains("[Qwen]")),
        "transcript should contain reviewer line: {:?}",
        transcript
    );
    Ok(())
}

struct QwenVisionTool {
    client: Client,
    endpoint: String,
    api_key: String,
}

impl QwenVisionTool {
    fn new(endpoint: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            api_key,
        }
    }
}

#[async_trait]
impl Tool for QwenVisionTool {
    fn name(&self) -> &'static str {
        "qwen.vision"
    }

    async fn call(
        &self,
        invocation: ToolInvocation,
        _ctx: &FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        let image_url = invocation
            .input
            .get("image_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentFlowError::Other(anyhow!("missing image_url")))?;
        let prompt = invocation
            .input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or(
                "请识别图片中的食物，并输出 JSON 格式 {\"foods\": [...], \"description\": \"...\"}",
            );

        let body = json!({
            "model": "qwen3-vl-plus",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": image_url}}
                ]
            }]
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .timeout(std::time::Duration::from_secs(40))
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no body>".to_string());
            return Err(AgentFlowError::Other(anyhow!(
                "vision request failed: {} {}",
                status,
                text
            )));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;
        let content = payload["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["content"].as_str())
            .ok_or_else(|| AgentFlowError::Other(anyhow!("missing content in vision response")))?;

        Ok(AgentMessage {
            id: new_message_id(),
            role: MessageRole::Tool,
            from: self.name().to_string(),
            to: None,
            content: content.to_string(),
            metadata: Some(payload),
        })
    }
}

struct QwenTextTool {
    client: Client,
    endpoint: String,
    api_key: String,
}

impl QwenTextTool {
    fn new(endpoint: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            api_key,
        }
    }
}

#[async_trait]
impl Tool for QwenTextTool {
    fn name(&self) -> &'static str {
        "qwen.text"
    }

    async fn call(
        &self,
        invocation: ToolInvocation,
        _ctx: &FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        let prompt = invocation
            .input
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentFlowError::Other(anyhow!("missing prompt")))?;
        let max_tokens = invocation
            .input
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(512);

        let body = json!({
            "model": "qwen3-max",
            "messages": [
                {"role": "system", "content": "你是一位经验丰富的营养师，擅长提供健康饮食建议。"},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": max_tokens
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .timeout(std::time::Duration::from_secs(40))
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "<no body>".to_string());
            return Err(AgentFlowError::Other(anyhow!(
                "text request failed: {} {}",
                status,
                text
            )));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| AgentFlowError::Other(e.into()))?;
        let content = payload["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["content"].as_str())
            .ok_or_else(|| AgentFlowError::Other(anyhow!("missing content in text response")))?;

        Ok(AgentMessage {
            id: new_message_id(),
            role: MessageRole::Tool,
            from: self.name().to_string(),
            to: None,
            content: content.to_string(),
            metadata: Some(payload),
        })
    }
}
