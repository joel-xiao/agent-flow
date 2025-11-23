use std::pin::Pin;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmRequest {
    #[serde(default)]
    pub system: Option<String>,
    pub user: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_base64: Option<String>,
}

fn default_temperature() -> f32 {
    0.2
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct LlmStreamChunk {
    pub content: String,
    pub done: bool,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>;

#[cfg(feature = "openai-client")]
#[derive(Clone, Debug)]
pub enum ApiFormat {
    OpenAI,
    Qwen,
    QwenVision,
}

