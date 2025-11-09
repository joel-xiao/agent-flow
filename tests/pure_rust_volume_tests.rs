#![allow(clippy::unwrap_used)]

use agentflow::agent::{Agent, AgentAction, AgentContext, AgentMessage};
use agentflow::error::AgentFlowError;
use agentflow::state::{FlowContext, MemoryStore};
use agentflow::{
    AgentRegistry, FlowBuilder, FlowExecutor, MessageRole, Result as FlowResult, ToolRegistry,
    register_agent,
};
use async_trait::async_trait;
use image::{Rgb, RgbImage};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

fn generate_test_image() -> RgbImage {
    let mut img = RgbImage::new(6, 4);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if y < 2 {
            *pixel = Rgb([220, 40, 40]); // red -> tomato
        } else if y == 2 {
            *pixel = Rgb([40, 200, 60]); // green -> broccoli
        } else {
            *pixel = Rgb([235, 200, 40]); // yellow -> egg
        }
        if x >= 4 && y >= 2 {
            *pixel = Rgb([245, 245, 240]); // white patch
        }
    }
    img
}

fn image_to_matrix(image: &RgbImage) -> Vec<Vec<[u8; 3]>> {
    let mut rows = Vec::new();
    for y in 0..image.height() {
        let mut row = Vec::new();
        for x in 0..image.width() {
            row.push(image.get_pixel(x, y).0);
        }
        rows.push(row);
    }
    rows
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
struct Detection {
    name: String,
    pixels: u32,
}

struct VisionAgent {
    next: String,
    log: Arc<Mutex<Vec<String>>>,
}

impl VisionAgent {
    fn new(next: impl Into<String>, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            next: next.into(),
            log,
        }
    }
}

#[async_trait]
impl Agent for VisionAgent {
    fn name(&self) -> &'static str {
        "vision_agent"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        _ctx: &AgentContext<'_>,
    ) -> FlowResult<AgentAction> {
        let matrix: Vec<Vec<[u8; 3]>> =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let mut counts = std::collections::HashMap::new();
        for row in &matrix {
            for [r, g, b] in row {
                let label = if *r > 210 && *g < 80 {
                    "番茄"
                } else if *g > 180 && *r < 100 {
                    "西兰花"
                } else if *r > 200 && *g > 180 && *b < 100 {
                    "鸡蛋"
                } else if *r > 230 && *g > 230 && *b > 220 {
                    "米饭"
                } else {
                    "其他"
                };
                *counts.entry(label).or_insert(0u32) += 1;
            }
        }

        let detections: Vec<Detection> = counts
            .into_iter()
            .filter(|(label, _)| *label != "其他")
            .map(|(name, pixels)| Detection {
                name: name.to_string(),
                pixels,
            })
            .collect();

        self.log
            .lock()
            .push(format!("[Vision] detections: {:?}", detections));

        let payload = json!({ "detections": detections });
        Ok(AgentAction::Next {
            target: self.next.clone(),
            message: AgentMessage {
                id: message.id,
                role: MessageRole::Agent,
                from: self.name().to_string(),
                to: Some(self.next.clone()),
                content: payload.to_string(),
                metadata: None,
            },
        })
    }
}

struct VolumeAgent {
    next: String,
    log: Arc<Mutex<Vec<String>>>,
}

impl VolumeAgent {
    fn new(next: impl Into<String>, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            next: next.into(),
            log,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VolumeDetail {
    name: String,
    volume_ml: f64,
}

#[async_trait]
impl Agent for VolumeAgent {
    fn name(&self) -> &'static str {
        "volume_agent"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        _ctx: &AgentContext<'_>,
    ) -> FlowResult<AgentAction> {
        let data: Value =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let detections: Vec<Detection> = serde_json::from_value(data["detections"].clone())
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        let mut details = Vec::new();
        for det in detections {
            let volume = (det.pixels as f64) * 1.8; // heuristic conversion
            details.push(VolumeDetail {
                name: det.name,
                volume_ml: volume,
            });
        }

        self.log
            .lock()
            .push(format!("[Volume] details: {:?}", details));

        let payload = json!({ "details": details });
        Ok(AgentAction::Next {
            target: self.next.clone(),
            message: AgentMessage {
                id: message.id,
                role: MessageRole::Agent,
                from: self.name().to_string(),
                to: Some(self.next.clone()),
                content: payload.to_string(),
                metadata: None,
            },
        })
    }
}

struct SummaryAgent;

#[async_trait]
impl Agent for SummaryAgent {
    fn name(&self) -> &'static str {
        "summary_agent"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        _ctx: &AgentContext<'_>,
    ) -> FlowResult<AgentAction> {
        let data: Value =
            serde_json::from_str(&message.content).map_err(|e| AgentFlowError::Other(e.into()))?;
        let details: Vec<VolumeDetail> = serde_json::from_value(data["details"].clone())
            .map_err(|e| AgentFlowError::Other(e.into()))?;

        let mut total = 0.0;
        let mut lines = Vec::new();
        for detail in &details {
            total += detail.volume_ml;
            lines.push(format!("{}≈{:.0} ml", detail.name, detail.volume_ml));
        }

        let summary = format!(
            "纯 Rust AgentFlow 估算: {}。总估算体积约 {:.0} ml。",
            lines.join("，"),
            total
        );

        Ok(AgentAction::Finish {
            message: Some(AgentMessage {
                id: message.id,
                role: MessageRole::Agent,
                from: self.name().to_string(),
                to: None,
                content: summary,
                metadata: None,
            }),
        })
    }
}

#[tokio::test]
async fn pure_rust_agentflow_volume_pipeline() -> FlowResult<()> {
    let image = generate_test_image();
    let matrix = image_to_matrix(&image);
    let initial_payload = serde_json::to_string(&matrix).expect("matrix serialization");

    let log = Arc::new(Mutex::new(Vec::new()));

    let mut agents = AgentRegistry::new();
    register_agent(
        "vision_agent",
        Arc::new(VisionAgent::new("volume_agent", Arc::clone(&log))),
        &mut agents,
    );
    register_agent(
        "volume_agent",
        Arc::new(VolumeAgent::new("summary_agent", Arc::clone(&log))),
        &mut agents,
    );
    register_agent("summary_agent", Arc::new(SummaryAgent), &mut agents);

    let mut builder = FlowBuilder::new("pure_rust_volume_flow");
    builder
        .add_agent_node("vision_agent", "vision_agent")
        .add_agent_node("volume_agent", "volume_agent")
        .add_agent_node("summary_agent", "summary_agent")
        .add_terminal_node("done")
        .set_start("vision_agent")
        .connect("vision_agent", "volume_agent")
        .connect("volume_agent", "summary_agent")
        .connect("summary_agent", "done");

    let flow = builder.build();
    let ctx_store: Arc<dyn agentflow::ContextStore> = Arc::new(MemoryStore::new());
    let flow_ctx = Arc::new(FlowContext::new(ctx_store));

    let executor = FlowExecutor::new(flow, agents, ToolRegistry::new());
    let initial_message = AgentMessage::user(initial_payload);
    let execution = executor.start(flow_ctx, initial_message).await?;

    assert_eq!(execution.flow_name, "pure_rust_volume_flow");
    assert_eq!(execution.last_node, "summary_agent");
    let summary = execution
        .last_message
        .expect("summary message should be present")
        .content;

    assert!(summary.contains("纯 Rust AgentFlow"));
    assert!(summary.contains("番茄"));
    assert!(summary.contains("西兰花"));
    assert!(summary.contains("鸡蛋"));

    Ok(())
}
