use std::collections::HashMap;
use std::fs;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use agentflow::agent::{
    Agent, AgentAction, AgentContext, AgentInput, AgentManifestBuilder, AgentMessage, AgentOutput,
    AgentPort, AgentPortSchema,
};
use agentflow::error::AgentFlowError;
use agentflow::state::{FlowContext, FlowScopeKind, MemoryStore};
use agentflow::tools::orchestrator::{ToolOrchestrator, ToolPipeline, ToolStep, ToolStrategy};
use agentflow::tools::{Tool, ToolInvocation, ToolManifestBuilder, ToolPort, ToolRegistry};
use agentflow::{
    AgentManifest, AgentRegistry, DecisionBranch, DecisionPolicy, FlowBuilder, FlowExecutor,
    FlowParameter, FlowVariable, JoinStrategy, MessageRole, PluginRegistry, Schema, SchemaKind,
    StructuredMessage, ToolManifest, ToolPortSchema, condition_state_equals,
    condition_state_not_equals, loop_condition_from_fn, register_agent, register_schema,
    schema_exports,
};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tempfile::tempdir;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MealRequest {
    time: String,
    items: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DietRequest {
    user: String,
    goal: String,
    strict: bool,
    calories: u32,
    meals: Vec<MealRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlanDraft {
    mode: String,
    target_calories: u32,
    meals: Vec<MealRequest>,
    baseline: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlanContribution {
    coach: String,
    adjustments: Vec<String>,
    focus: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MacroPlan {
    total_calories: u32,
    protein: u32,
    carbs: u32,
    fat: u32,
    guidance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HydrationPlan {
    water_ml: u32,
    electrolyte: bool,
    checkpoints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MotivationResponse {
    headline: String,
    affirmations: Vec<String>,
    reminder: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FinalSummary {
    status: String,
    coach_notes: Vec<String>,
    pipelines: HashMap<String, Value>,
}

struct IntakeAgent {
    manifest: AgentManifest,
}

#[async_trait]
impl Agent for IntakeAgent {
    fn name(&self) -> &'static str {
        "intake_agent"
    }

    async fn on_start(&self, ctx: &AgentContext<'_>) -> agentflow::Result<()> {
        let serialized = serde_json::to_string(&self.manifest)
            .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
        ctx.session().set("diet.manifest", serialized).await?;
        Ok(())
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let AgentInput { value, .. } = AgentInput::<DietRequest>::try_from_message(message)?;
        let mode = if value.strict { "strict" } else { "flex" };
        ctx.session().set("diet.user", &value.user).await?;
        ctx.session().set("diet.goal", &value.goal).await?;
        ctx.session().set("diet.mode", mode).await?;
        ctx.flow_ctx
            .store()
            .set("diet.mode", mode.to_string())
            .await?;
        ctx.flow_ctx
            .store()
            .set("diet.target_calories", value.calories.to_string())
            .await?;
        ctx.variables()
            .set_global("active_plan_style", mode)
            .await?;

        let baseline = value
            .meals
            .iter()
            .map(|meal| format!("{}: {}", meal.time, meal.items.join(", ")))
            .collect::<Vec<_>>();
        let draft = PlanDraft {
            mode: mode.to_string(),
            target_calories: value.calories,
            meals: value.meals.clone(),
            baseline,
        };
        let draft_json = serde_json::to_string(&draft)
            .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
        ctx.flow_ctx
            .store()
            .set("diet.plan_draft", draft_json)
            .await?;

        let output = AgentOutput {
            role: MessageRole::Agent,
            from: self.name().to_string(),
            to: Some("plan_gate".to_string()),
            value: draft,
            metadata: Some(json!({"source": "intake_agent"})),
        };
        Ok(AgentAction::Next {
            target: "plan_gate".to_string(),
            message: output.into_message()?,
        })
    }
}

struct PlanCoachAgent {
    coach: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Agent for PlanCoachAgent {
    fn name(&self) -> &'static str {
        self.coach
    }

    async fn on_message(
        &self,
        _message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let draft_json = ctx
            .flow_ctx
            .store()
            .get("diet.plan_draft")
            .await?
            .ok_or_else(|| AgentFlowError::Context("missing draft".into()))?;
        let draft: PlanDraft = serde_json::from_str(&draft_json)
            .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
        let emphasis = if self.coach == "strict_coach" {
            "严格控制碳水"
        } else {
            "灵活分配热量"
        };
        let adjustments = draft
            .baseline
            .iter()
            .map(|meal| format!("{} -> {}", meal, emphasis))
            .collect::<Vec<_>>();
        let note = format!("{}:{}", self.coach, adjustments.join(" | "));
        ctx.variables()
            .set_global(format!("plan.{}", self.coach), note.clone())
            .await?;
        self.log.lock().push(format!(
            "{} prepared {} entries",
            self.coach,
            adjustments.len()
        ));

        let contribution = PlanContribution {
            coach: self.coach.to_string(),
            adjustments,
            focus: emphasis.to_string(),
        };

        let output = AgentOutput {
            role: MessageRole::Agent,
            from: self.coach.to_string(),
            to: Some("plan_join".to_string()),
            value: contribution,
            metadata: Some(json!({"coach": self.coach})),
        };

        Ok(AgentAction::Next {
            target: "plan_join".to_string(),
            message: output.into_message()?,
        })
    }
}

struct HabitCoachAgent {
    loop_log: Arc<RwLock<Vec<String>>>,
}

#[async_trait]
impl Agent for HabitCoachAgent {
    fn name(&self) -> &'static str {
        "habit_coach"
    }

    async fn on_message(
        &self,
        step_message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let mut log = self.loop_log.write().await;
        log.push(format!("received: {}", step_message.content));
        let iteration = log.len();
        drop(log);
        let history_marker = AgentMessage::system(format!("habit_iteration_{}", iteration));
        ctx.flow_ctx.push_message(history_marker);
        Ok(AgentAction::Continue {
            message: Some(AgentMessage::system(format!(
                "reinforcement_round_{}",
                iteration
            ))),
        })
    }
}

struct ReviewAgent {
    summary: Arc<Mutex<Option<FinalSummary>>>,
}

#[async_trait]
impl Agent for ReviewAgent {
    fn name(&self) -> &'static str {
        "review_agent"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> agentflow::Result<AgentAction> {
        let response: MotivationResponse = serde_json::from_str(&message.content)
            .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
        let macros = ctx
            .flow_ctx
            .store()
            .get("diet.macros_total")
            .await?
            .unwrap_or_else(|| "0".into())
            .parse::<u32>()
            .unwrap_or_default();
        let plan_notes = vec![
            ctx.variables()
                .get("plan.strict_coach")
                .await
                .unwrap_or_else(|| "strict_coach:未执行".into()),
            ctx.variables()
                .get("plan.flex_coach")
                .await
                .unwrap_or_else(|| "flex_coach:未执行".into()),
        ];

        let mut pipelines = HashMap::new();
        pipelines.insert("motivation".into(), json!(&response));
        pipelines.insert(
            "history_count".into(),
            json!({
                "messages": ctx.flow_ctx.history().len(),
                "macros_total": macros
            }),
        );

        let summary = FinalSummary {
            status: if macros <= 0 {
                "incomplete".into()
            } else {
                "ready".into()
            },
            coach_notes: plan_notes,
            pipelines,
        };

        let serialized = serde_json::to_string(&summary)
            .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
        ctx.session().set("diet.summary", serialized).await?;
        *self.summary.lock() = Some(summary.clone());

        let output = AgentOutput {
            role: MessageRole::Agent,
            from: self.name().to_string(),
            to: Some("finish".to_string()),
            value: summary,
            metadata: Some(json!({"stage": "review"})),
        };

        Ok(AgentAction::Finish {
            message: Some(output.into_message()?),
        })
    }
}

struct MacroEstimatorTool;

#[async_trait]
impl Tool for MacroEstimatorTool {
    fn name(&self) -> &'static str {
        "nutrition.macros"
    }

    async fn call(
        &self,
        invocation: ToolInvocation,
        ctx: &FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        #[derive(Deserialize)]
        struct MacroInput {
            calories: u32,
            strict: bool,
        }

        let payload: MacroInput = serde_json::from_value(invocation.input)
            .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
        let protein = (payload.calories as f32 * if payload.strict { 0.4 } else { 0.3 }) as u32;
        let carbs = (payload.calories as f32 * if payload.strict { 0.3 } else { 0.45 }) as u32;
        let fat = payload.calories.saturating_sub(protein + carbs);
        ctx.store()
            .set("diet.macros_total", payload.calories.to_string())
            .await?;
        let guidance = if payload.strict {
            "保持高蛋白，晚间碳水 <20g"
        } else {
            "允许适度碳水，关注总体赤字"
        };
        let structured = StructuredMessage::new(MacroPlan {
            total_calories: payload.calories,
            protein,
            carbs,
            fat,
            guidance: guidance.into(),
        })
        .with_schema("diet.plan.macros");
        structured.into_agent_message(MessageRole::Tool, self.name(), None)
    }
}

struct HydrationPlannerTool;

#[async_trait]
impl Tool for HydrationPlannerTool {
    fn name(&self) -> &'static str {
        "hydration.plan"
    }

    async fn call(
        &self,
        _invocation: ToolInvocation,
        _ctx: &FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        StructuredMessage::new(HydrationPlan {
            water_ml: 2400,
            electrolyte: true,
            checkpoints: vec![
                "餐前30分钟".to_string(),
                "运动前".to_string(),
                "睡前60分钟".to_string(),
            ],
        })
        .with_schema("diet.plan.hydration")
        .into_agent_message(MessageRole::Tool, self.name(), None)
    }
}

struct MotivationPrimaryTool {
    attempts: AtomicUsize,
}

#[async_trait]
impl Tool for MotivationPrimaryTool {
    fn name(&self) -> &'static str {
        "motivation.primary"
    }

    async fn call(
        &self,
        _invocation: ToolInvocation,
        _ctx: &FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(AgentFlowError::Other(anyhow!(
            "primary provider offline (attempt {})",
            attempt + 1
        )))
    }
}

struct MotivationFallbackTool;

#[async_trait]
impl Tool for MotivationFallbackTool {
    fn name(&self) -> &'static str {
        "motivation.backup"
    }

    async fn call(
        &self,
        invocation: ToolInvocation,
        _ctx: &FlowContext,
    ) -> agentflow::Result<AgentMessage> {
        let tone = invocation
            .input
            .get("tone")
            .and_then(Value::as_str)
            .unwrap_or("balanced")
            .to_string();
        StructuredMessage::new(MotivationResponse {
            headline: format!("保持{}节奏，你正在打造长期可持续减脂方案", tone),
            affirmations: vec![
                "记录每次训练和饮食".into(),
                "补水时顺带做深呼吸".into(),
                "睡前提前准备食材".into(),
            ],
            reminder: "不要因一次放纵打乱节奏".into(),
        })
        .with_schema("diet.plan.motivation")
        .into_agent_message(MessageRole::Tool, self.name(), None)
    }
}

fn macro_manifest() -> ToolManifest {
    ToolManifestBuilder::new("nutrition.macros")
        .description("估算宏量营养素配比")
        .input(ToolPort::new("request").with_schema(ToolPortSchema::new().with_type("MacroInput")))
        .output(ToolPort::new("plan").with_schema(ToolPortSchema::new().with_type("MacroPlan")))
        .build()
}

fn hydration_manifest() -> ToolManifest {
    ToolManifestBuilder::new("hydration.plan")
        .description("生成补水计划")
        .output(ToolPort::new("plan").with_schema(ToolPortSchema::new().with_type("HydrationPlan")))
        .build()
}

fn motivation_manifest(name: &str) -> ToolManifest {
    ToolManifestBuilder::new(name)
        .description("生成激励文案")
        .input(ToolPort::new("tone"))
        .output(
            ToolPort::new("message")
                .with_schema(ToolPortSchema::new().with_type("MotivationResponse")),
        )
        .build()
}

fn register_diet_schemas() -> Result<()> {
    let mut request_props = HashMap::new();
    request_props.insert(
        "user".into(),
        Schema::new(SchemaKind::String).with_description("用户姓名"),
    );
    request_props.insert(
        "goal".into(),
        Schema::new(SchemaKind::String).with_description("目标说明"),
    );
    request_props.insert("strict".into(), Schema::new(SchemaKind::Boolean));
    request_props.insert("calories".into(), Schema::new(SchemaKind::Integer));
    request_props.insert(
        "meals".into(),
        Schema::new(SchemaKind::Array {
            items: Box::new(Schema::new(SchemaKind::Object {
                properties: HashMap::from([
                    ("time".into(), Schema::new(SchemaKind::String)),
                    (
                        "items".into(),
                        Schema::new(SchemaKind::Array {
                            items: Box::new(Schema::new(SchemaKind::String)),
                        }),
                    ),
                ]),
                required: vec!["time".into(), "items".into()],
                additional: false,
            })),
        }),
    );
    register_schema(
        "diet.request",
        Schema::new(SchemaKind::Object {
            properties: request_props,
            required: vec![
                "user".into(),
                "goal".into(),
                "calories".into(),
                "meals".into(),
            ],
            additional: false,
        }),
    );

    register_schema(
        "diet.plan.macros",
        Schema::new(SchemaKind::Object {
            properties: HashMap::from([
                ("total_calories".into(), Schema::new(SchemaKind::Integer)),
                ("protein".into(), Schema::new(SchemaKind::Integer)),
                ("carbs".into(), Schema::new(SchemaKind::Integer)),
                ("fat".into(), Schema::new(SchemaKind::Integer)),
                ("guidance".into(), Schema::new(SchemaKind::String)),
            ]),
            required: vec![
                "total_calories".into(),
                "protein".into(),
                "carbs".into(),
                "fat".into(),
            ],
            additional: false,
        }),
    );

    register_schema(
        "diet.plan.hydration",
        Schema::new(SchemaKind::Object {
            properties: HashMap::from([
                ("water_ml".into(), Schema::new(SchemaKind::Integer)),
                ("electrolyte".into(), Schema::new(SchemaKind::Boolean)),
                (
                    "checkpoints".into(),
                    Schema::new(SchemaKind::Array {
                        items: Box::new(Schema::new(SchemaKind::String)),
                    }),
                ),
            ]),
            required: vec!["water_ml".into()],
            additional: false,
        }),
    );

    register_schema(
        "diet.plan.motivation",
        Schema::new(SchemaKind::Object {
            properties: HashMap::from([
                ("headline".into(), Schema::new(SchemaKind::String)),
                (
                    "affirmations".into(),
                    Schema::new(SchemaKind::Array {
                        items: Box::new(Schema::new(SchemaKind::String)),
                    }),
                ),
                ("reminder".into(), Schema::new(SchemaKind::String)),
            ]),
            required: vec!["headline".into(), "affirmations".into()],
            additional: false,
        }),
    );
    Ok(())
}

#[tokio::test]
async fn complex_diet_flow_masterplan_covers_full_stack() -> Result<()> {
    register_diet_schemas()?;

    let temp = tempdir()?;
    let plugin_dir = temp.path().join("acme_diet");
    fs::create_dir(&plugin_dir)?;
    let plugin = json!({
        "name": "acme.diet.bundle",
        "version": "0.1.0",
        "kind": "schema",
        "schemas": ["diet.request", "diet.plan.macros"],
        "description": "Demo diet plugin"
    });
    fs::write(plugin_dir.join("plugin.json"), plugin.to_string())?;

    let mut plugin_registry = PluginRegistry::new();
    plugin_registry.load_directory(temp.path())?;
    let manifests: Vec<_> = plugin_registry.manifests().cloned().collect();
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].name, "acme.diet.bundle");

    let mut base_tool_registry = ToolRegistry::new();
    plugin_registry.initialize(&mut base_tool_registry)?;

    let macro_tool: Arc<dyn Tool> = Arc::new(MacroEstimatorTool);
    let hydration_tool: Arc<dyn Tool> = Arc::new(HydrationPlannerTool);
    let motivation_primary: Arc<dyn Tool> = Arc::new(MotivationPrimaryTool {
        attempts: AtomicUsize::new(0),
    });
    let motivation_backup: Arc<dyn Tool> = Arc::new(MotivationFallbackTool);

    base_tool_registry.register_with_manifest(Arc::clone(&macro_tool), macro_manifest())?;
    base_tool_registry.register_with_manifest(Arc::clone(&hydration_tool), hydration_manifest())?;
    base_tool_registry.register_with_manifest(
        Arc::clone(&motivation_primary),
        motivation_manifest("motivation.primary"),
    )?;
    base_tool_registry.register_with_manifest(
        Arc::clone(&motivation_backup),
        motivation_manifest("motivation.backup"),
    )?;

    let mut orchestrator = ToolOrchestrator::new(ToolRegistry::new());
    orchestrator
        .registry_mut()
        .register_with_manifest(Arc::clone(&macro_tool), macro_manifest())?;
    orchestrator
        .registry_mut()
        .register_with_manifest(Arc::clone(&hydration_tool), hydration_manifest())?;
    orchestrator.registry_mut().register_with_manifest(
        Arc::clone(&motivation_primary),
        motivation_manifest("motivation.primary"),
    )?;
    orchestrator.registry_mut().register_with_manifest(
        Arc::clone(&motivation_backup),
        motivation_manifest("motivation.backup"),
    )?;

    orchestrator.register_pipeline(ToolPipeline::new(
        "nutrition.pipeline",
        ToolStrategy::Sequential(vec![
            ToolStep::new(
                "nutrition.macros",
                json!({ "calories": 1650, "strict": true }),
            )
            .with_timeout(Duration::from_millis(50))
            .with_name("macros"),
            ToolStep::new("hydration.plan", json!({ "activity": "resistance" }))
                .with_retries(1)
                .with_name("hydration"),
        ]),
    ))?;

    orchestrator.register_pipeline(ToolPipeline::new(
        "motivation.pipeline",
        ToolStrategy::Fallback(vec![
            ToolStep::new("motivation.primary", json!({ "tone": "strict" }))
                .with_timeout(Duration::from_millis(10)),
            ToolStep::new("motivation.backup", json!({ "tone": "strict" })),
        ]),
    ))?;

    let manifest = AgentManifestBuilder::new("intake_agent")
        .description("解析用户输入并准备计划草稿")
        .input(
            AgentPort::new("request")
                .with_schema(AgentPortSchema::new().with_type("DietRequest"))
                .with_description("结构化饮食请求"),
        )
        .output(AgentPort::new("draft").with_description("计划草稿"))
        .tool("nutrition.macros")
        .capability("diet:intake")
        .build();

    let intake_agent: Arc<dyn Agent> = Arc::new(IntakeAgent {
        manifest: manifest.clone(),
    });
    let coach_log = Arc::new(Mutex::new(Vec::new()));
    let strict_coach: Arc<dyn Agent> = Arc::new(PlanCoachAgent {
        coach: "strict_coach",
        log: Arc::clone(&coach_log),
    });
    let flex_coach: Arc<dyn Agent> = Arc::new(PlanCoachAgent {
        coach: "flex_coach",
        log: Arc::clone(&coach_log),
    });
    let habit_log = Arc::new(RwLock::new(Vec::new()));
    let habit_coach: Arc<dyn Agent> = Arc::new(HabitCoachAgent {
        loop_log: Arc::clone(&habit_log),
    });
    let review_summary = Arc::new(Mutex::new(None));
    let review_agent: Arc<dyn Agent> = Arc::new(ReviewAgent {
        summary: Arc::clone(&review_summary),
    });

    let mut agents = AgentRegistry::new();
    register_agent("intake_agent", Arc::clone(&intake_agent), &mut agents);
    register_agent("strict_coach", Arc::clone(&strict_coach), &mut agents);
    register_agent("flex_coach", Arc::clone(&flex_coach), &mut agents);
    register_agent("habit_coach", Arc::clone(&habit_coach), &mut agents);
    register_agent("review_agent", Arc::clone(&review_agent), &mut agents);

    let mut flow_builder = FlowBuilder::new("complex_diet_coaching");
    let loop_condition = loop_condition_from_fn(|ctx| {
        ctx.history()
            .iter()
            .filter(|msg| msg.content.starts_with("habit_iteration"))
            .count()
            < 3
    });

    flow_builder
        .with_parameter(FlowParameter::input::<DietRequest>("request"))
        .declare_variable(FlowVariable::new(
            "active_plan_style",
            FlowScopeKind::Global,
        ))
        .add_agent_node("intake", "intake_agent")
        .add_decision_node(
            "plan_gate",
            DecisionPolicy::AllMatches,
            vec![
                DecisionBranch {
                    name: Some("strict".into()),
                    condition: Some(condition_state_equals("diet.mode", "strict")),
                    target: "strict_coach".into(),
                },
                DecisionBranch {
                    name: Some("flex".into()),
                    condition: Some(condition_state_not_equals("diet.mode", "strict")),
                    target: "flex_coach".into(),
                },
            ],
        )
        .add_agent_node("strict_coach", "strict_coach")
        .add_agent_node("flex_coach", "flex_coach")
        .add_join_node(
            "plan_join",
            JoinStrategy::Any,
            vec!["strict_coach".into(), "flex_coach".into()],
        )
        .add_tool_node("plan_tools", "nutrition.pipeline")
        .add_agent_node("habit_coach", "habit_coach")
        .add_loop_node(
            "habit_loop",
            "habit_coach",
            Some(loop_condition),
            None,
            Some("motivation_pipeline".to_string()),
        )
        .add_tool_node("motivation_pipeline", "motivation.pipeline")
        .add_agent_node("review", "review_agent")
        .add_terminal_node("finish")
        .set_start("intake")
        .connect("intake", "plan_gate")
        .connect("plan_join", "plan_tools")
        .connect("plan_tools", "habit_loop")
        .connect("motivation_pipeline", "review")
        .connect("review", "finish")
        .connect("strict_coach", "plan_join")
        .connect("flex_coach", "plan_join")
        .connect_loop("habit_loop", "habit_coach", Some("motivation_pipeline"));

    let flow = flow_builder.build();

    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let ctx = Arc::new(FlowContext::new(Arc::clone(&ctx_store)));

    let executor = FlowExecutor::new(flow, agents, base_tool_registry)
        .with_max_concurrency(4)
        .with_tool_orchestrator(Arc::new(orchestrator));

    let initial_message = StructuredMessage::new(DietRequest {
        user: "TestUser".into(),
        goal: "三个月减脂 6kg".into(),
        strict: true,
        calories: 1650,
        meals: vec![
            MealRequest {
                time: "07:30".into(),
                items: vec!["燕麦".to_string(), "鸡蛋".to_string()],
            },
            MealRequest {
                time: "12:30".into(),
                items: vec!["鸡胸肉".to_string(), "蔬菜沙拉".to_string()],
            },
            MealRequest {
                time: "18:30".into(),
                items: vec!["三文鱼".to_string(), "糙米".to_string()],
            },
        ],
    })
    .with_schema("diet.request")
    .with_metadata(json!({ "entry": "unit_test" }))
    .into_agent_message(MessageRole::User, "diet.client", None)?;

    let result = executor
        .start(Arc::clone(&ctx), initial_message)
        .await
        .context("flow execution")?;

    assert_eq!(result.flow_name, "complex_diet_coaching");
    assert_eq!(result.last_node, "review");
    assert!(result.errors.is_empty());

    let session_summary = ctx
        .session()
        .get("diet.summary")
        .await?
        .context("missing summary")?;
    let summary: FinalSummary = serde_json::from_str(&session_summary)?;
    assert_eq!(summary.status, "ready");
    assert!(
        summary
            .coach_notes
            .iter()
            .any(|note| note.contains("strict_coach"))
    );
    assert!(summary.pipelines.contains_key("motivation"));

    let review_guard = review_summary.lock();
    assert!(review_guard.is_some());
    drop(review_guard);

    assert!(
        coach_log
            .lock()
            .iter()
            .any(|entry| entry.contains("strict_coach"))
    );
    assert!(
        schema_exports()
            .iter()
            .any(|entry| entry.name == "diet.request")
    );

    Ok(())
}
