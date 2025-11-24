pub mod client;
#[cfg(feature = "openai-client")]
pub mod config;
pub mod echo;
#[cfg(feature = "openai-client")]
pub mod extended;
#[cfg(feature = "openai-client")]
pub mod http;
pub mod types;

pub use client::{DynLlmClient, LlmClient};
pub use echo::LocalEchoClient;
#[cfg(feature = "openai-client")]
pub use types::ApiFormat;
pub use types::{LlmMessage, LlmRequest, LlmResponse, LlmStreamChunk};

#[cfg(feature = "openai-client")]
pub use config::ApiEndpointConfig;

// 导出JSON配置相关功能（用于高级用例）
#[cfg(feature = "openai-client")]
pub use extended::{
    ApiBuilder, ApiCallBuilder, JsonApiClient, UniversalApiClient,
};

// 导出核心HTTP客户端（推荐用于LLM调用）
#[cfg(feature = "openai-client")]
pub use http::GenericHttpClient;
#[cfg(feature = "openai-client")]
pub use http::*;
