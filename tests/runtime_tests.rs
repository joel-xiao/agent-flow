use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::time::{Duration, sleep};

use agentflow::agent::builtin::register_builtin_agent_factories;
use agentflow::autogen::AutogenConfig;
use agentflow::state::MemoryStore;
use agentflow::tools::register_builtin_tool_factories;
use agentflow::{
    Agent, AgentAction, AgentContext, AgentFactoryRegistry, AgentMessage, AgentRegistry,
    FlowBuilder, FlowContext, FlowExecutor, FlowRegistry, ToolFactoryRegistry, ToolRegistry,
    register_agent,
};

struct RecorderAgent {
    name: &'static str,
    next: Option<String>,
    log: Arc<Mutex<Vec<String>>>,
    delay_ms: u64,
}

impl RecorderAgent {
    fn new(name: &'static str, next: Option<String>, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            name,
            next,
            log,
            delay_ms: 0,
        }
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

#[async_trait::async_trait]
impl Agent for RecorderAgent {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        _ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        if self.delay_ms > 0 {
            sleep(Duration::from_millis(self.delay_ms)).await;
        }
        self.log
            .lock()
            .push(format!("{}:{}", self.name, message.content));
        if let Some(next) = &self.next {
            Ok(AgentAction::Next {
                target: next.clone(),
                message,
            })
        } else {
            Ok(AgentAction::Finish {
                message: Some(AgentMessage::system("finished")),
            })
        }
    }
}

#[tokio::test]
async fn flow_executor_runs_linear_flow() -> anyhow::Result<()> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut agents = AgentRegistry::new();
    register_agent(
        "start",
        Arc::new(RecorderAgent::new(
            "start",
            Some("worker".into()),
            Arc::clone(&log),
        )),
        &mut agents,
    );
    register_agent(
        "worker",
        Arc::new(RecorderAgent::new(
            "worker",
            Some("finish".into()),
            Arc::clone(&log),
        )),
        &mut agents,
    );

    let mut flow_builder = FlowBuilder::new("linear");
    flow_builder
        .add_agent_node("start", "start")
        .add_agent_node("worker", "worker")
        .add_terminal_node("finish")
        .set_start("start")
        .connect("start", "worker")
        .connect("worker", "finish");

    let flow = flow_builder.build();
    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, ToolRegistry::new());
    let result = executor.start(ctx, AgentMessage::user("ping")).await?;

    // 验证流程终点与最终节点名称
    assert_eq!(result.flow_name, "linear");
    assert_eq!(result.last_node, "finish");
    let history = log.lock();
    // 记录的消息顺序应与节点顺序一致
    assert_eq!(history.len(), 2);
    assert_eq!(history[0], "start:ping");
    assert_eq!(history[1], "worker:ping");
    Ok(())
}

#[tokio::test]
async fn flow_executor_concurrent_branches() -> anyhow::Result<()> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut agents = AgentRegistry::new();

    register_agent(
        "branch",
        Arc::new(BranchAgent::new(
            vec!["worker_a".into(), "worker_b".into()],
            Arc::clone(&log),
        )),
        &mut agents,
    );
    register_agent(
        "worker_a",
        Arc::new(
            RecorderAgent::new("worker_a", Some("finish_a".into()), Arc::clone(&log))
                .with_delay(50),
        ),
        &mut agents,
    );
    register_agent(
        "worker_b",
        Arc::new(
            RecorderAgent::new("worker_b", Some("finish_b".into()), Arc::clone(&log))
                .with_delay(10),
        ),
        &mut agents,
    );
    register_agent(
        "finish_a",
        Arc::new(RecorderAgent::new("finish_a", None, Arc::clone(&log))),
        &mut agents,
    );
    register_agent(
        "finish_b",
        Arc::new(RecorderAgent::new("finish_b", None, Arc::clone(&log))),
        &mut agents,
    );

    let mut flow_builder = FlowBuilder::new("branch");
    flow_builder
        .add_agent_node("branch", "branch")
        .add_agent_node("worker_a", "worker_a")
        .add_agent_node("worker_b", "worker_b")
        .add_agent_node("finish_a", "finish_a")
        .add_agent_node("finish_b", "finish_b")
        .add_terminal_node("done")
        .set_start("branch")
        .connect("branch", "worker_a")
        .connect("branch", "worker_b")
        .connect("worker_a", "finish_a")
        .connect("worker_b", "finish_b")
        .connect("finish_a", "done")
        .connect("finish_b", "done");

    let flow = flow_builder.build();

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));
    let executor = FlowExecutor::new(flow, agents, ToolRegistry::new()).with_max_concurrency(4);
    let result = executor.start(ctx, AgentMessage::user("branch")).await?;

    // 并行流程应在某个完成节点结束
    assert_eq!(result.flow_name, "branch");
    assert!(
        ["finish_a", "finish_b", "done"].contains(&result.last_node.as_str()),
        "unexpected terminal node {}",
        result.last_node
    );
    let history = log.lock();
    // 两条分支都应该被调度执行
    assert!(history.contains(&"worker_a:branch".to_string()));
    assert!(history.contains(&"worker_b:branch".to_string()));
    Ok(())
}

struct BranchAgent {
    targets: Vec<String>,
    log: Arc<Mutex<Vec<String>>>,
}

impl BranchAgent {
    fn new(targets: Vec<String>, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self { targets, log }
    }
}

#[async_trait::async_trait]
impl Agent for BranchAgent {
    fn name(&self) -> &'static str {
        "branch"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        _ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        self.log.lock().push(format!("branch:{}", message.content));
        let mut branches = HashMap::new();
        for target in &self.targets {
            branches.insert(target.clone(), message.clone());
        }
        Ok(AgentAction::Branch { branches })
    }
}

#[test]
fn autogen_registers_agents_tools_and_flows() -> anyhow::Result<()> {
    let yaml = r#"
agents:
  - name: user_proxy
    factory: user_proxy
    config:
      next: coder
  - name: coder
    factory: coder
    config:
      reviewer: reviewer
  - name: reviewer
    factory: reviewer
    config:
      coder: coder

tools:
  - factory: echo

flows:
  - name: code_review
    start: user_proxy
    nodes:
      - name: user_proxy
        agent: user_proxy
      - name: coder
        agent: coder
      - name: reviewer
        agent: reviewer
      - name: finish
        terminal: true
    transitions:
      - from: user_proxy
        to: coder
      - from: coder
        to: reviewer
      - from: reviewer
        to: finish
"#;

    let config = AutogenConfig::from_reader(yaml.as_bytes())?;
    let mut agent_factories = AgentFactoryRegistry::new();
    register_builtin_agent_factories(&mut agent_factories);
    let mut tool_factories = ToolFactoryRegistry::new();
    register_builtin_tool_factories(&mut tool_factories);

    let mut agents = AgentRegistry::new();
    let mut tools = ToolRegistry::new();
    let mut flows = FlowRegistry::new();

    config.register_all(
        &agent_factories,
        &mut agents,
        &tool_factories,
        &mut tools,
        &mut flows,
    )?;

    // YAML 描述中所有实体均应成功注册到对应 Registry
    assert!(agents.contains_key("coder"));
    assert!(tools.get("echo").is_some());
    assert!(flows.get("code_review").is_some());
    Ok(())
}

#[cfg(feature = "openai-client")]
mod qwen_integration {
    use super::*;
    use agentflow::MessageRole;
    use agentflow::agent::builtin::ToolInvokerAgent;
    fn new_message_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("qwen-msg-{}-{}", now.as_secs(), now.subsec_nanos())
    }

    use agentflow::tools::Tool;
    use anyhow::anyhow;
    use reqwest::Client;
    use serde_json::json;

    #[tokio::test]
    async fn multi_agent_qwen_flow() -> anyhow::Result<()> {
        let api_key =
            match std::env::var("QWEN_API_KEY").or_else(|_| std::env::var("DASHSCOPE_API_KEY")) {
                Ok(key) => key,
                Err(_) => {
                    // 未设置密钥时跳过此集成测试
                    eprintln!("Skipping Qwen test: QWEN_API_KEY not set");
                    return Ok(());
                }
            };

        let endpoint = std::env::var("QWEN_BASE_URL").unwrap_or_else(|_| {
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string()
        });
        let model = std::env::var("QWEN_MODEL").unwrap_or_else(|_| "qwen-max".to_string());
        let system_prompt = std::env::var("QWEN_SYSTEM_PROMPT")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                "You are a helpful assistant specialising in concise Rust code examples."
                    .to_string()
            });

        let log = Arc::new(Mutex::new(Vec::new()));
        let mut agents = AgentRegistry::new();
        register_agent(
            "starter",
            Arc::new(RecorderAgent::new(
                "starter",
                Some("llm_invoker".into()),
                Arc::clone(&log),
            )),
            &mut agents,
        );
        register_agent(
            "llm_invoker",
            Arc::new(ToolInvokerAgent::new("llm.qwen", Some("collector".into()))),
            &mut agents,
        );
        register_agent(
            "collector",
            Arc::new(CollectorAgent::new(Arc::clone(&log))),
            &mut agents,
        );

        let mut flow_builder = FlowBuilder::new("qwen_flow");
        flow_builder
            .add_agent_node("starter", "starter")
            .add_agent_node("llm_invoker", "llm_invoker")
            .add_agent_node("collector", "collector")
            .add_terminal_node("finish")
            .set_start("starter")
            .connect("starter", "llm_invoker")
            .connect("llm_invoker", "collector")
            .connect("collector", "finish");

        let flow = flow_builder.build();

        let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
        let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(QwenTool::new(
            endpoint,
            api_key,
            model,
            system_prompt,
        )));

        let executor = FlowExecutor::new(flow, agents, tools);
        let prompt = "请简要说明 Rust 中 async/await 的作用，并给出一个示例。";
        let result = executor.start(ctx, AgentMessage::user(prompt)).await?;

        let history = log.lock();
        println!("\n[Qwen Conversation Transcript]");
        for entry in history.iter() {
            println!("  {}", entry);
        }
        // collector 应当接收来自千问的回答
        assert!(
            history.iter().any(|entry| entry.starts_with("collector:")),
            "expected collector to receive output, history: {:?}",
            *history
        );
        if let Some(last) = &result.last_message {
            // 回答包含 Rust/async 关键词即可视为符合预期
            assert!(
                last.content.to_lowercase().contains("rust")
                    || last.content.contains("示例")
                    || last.content.contains("async"),
                "unexpected response content: {}",
                last.content
            );
        }
        Ok(())
    }

    struct CollectorAgent {
        log: Arc<Mutex<Vec<String>>>,
    }

    impl CollectorAgent {
        fn new(log: Arc<Mutex<Vec<String>>>) -> Self {
            Self { log }
        }
    }

    #[async_trait::async_trait]
    impl Agent for CollectorAgent {
        fn name(&self) -> &'static str {
            "collector"
        }

        async fn on_message(
            &self,
            message: AgentMessage,
            _ctx: &AgentContext<'_>,
        ) -> agentflow::Result<AgentAction> {
            self.log
                .lock()
                .push(format!("collector:{}", message.content));
            Ok(AgentAction::Finish {
                message: Some(message),
            })
        }
    }

    struct QwenTool {
        client: Client,
        endpoint: String,
        api_key: String,
        model: String,
        system_prompt: String,
    }

    impl QwenTool {
        fn new(endpoint: String, api_key: String, model: String, system_prompt: String) -> Self {
            Self {
                client: Client::new(),
                endpoint,
                api_key,
                model,
                system_prompt,
            }
        }

        fn extract_input(input: &serde_json::Value) -> String {
            if let Some(s) = input.as_str() {
                return s.to_string();
            }
            if let Some(obj) = input.as_object() {
                if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                    return content.to_string();
                }
            }
            input.to_string()
        }
    }

    #[async_trait::async_trait]
    impl Tool for QwenTool {
        fn name(&self) -> &'static str {
            "llm.qwen"
        }

        async fn call(
            &self,
            invocation: agentflow::ToolInvocation,
            _ctx: &FlowContext,
        ) -> agentflow::Result<AgentMessage> {
            let user_input = Self::extract_input(&invocation.input);
            let body = json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": self.system_prompt},
                    {"role": "user", "content": user_input}
                ],
                "temperature": 0.2
            });

            let response = self
                .client
                .post(&self.endpoint)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| agentflow::AgentFlowError::Other(e.into()))?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<no body>".to_string());
                return Err(agentflow::AgentFlowError::Other(anyhow!(
                    "Qwen request failed: {} {}",
                    status,
                    text
                )));
            }

            let payload: serde_json::Value = response
                .json()
                .await
                .map_err(|e| agentflow::AgentFlowError::Other(e.into()))?;
            let content = payload["choices"]
                .get(0)
                .and_then(|choice| choice["message"]["content"].as_str())
                .ok_or_else(|| {
                    agentflow::AgentFlowError::Other(anyhow!("missing content in Qwen response"))
                })?
                .to_string();

            Ok(AgentMessage {
                id: new_message_id(),
                role: MessageRole::Tool,
                from: self.name().to_string(),
                to: None,
                content,
                metadata: Some(payload),
            })
        }
    }
}
