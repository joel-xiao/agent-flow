//! HTTP 客户端实现模块
//!
//! 此模块提供统一的 HTTP 客户端实现，用于与各种 LLM API 服务通信。
//!
//! 核心组件：
//! - `GenericHttpClient`: 统一的 HTTP 客户端，支持多种 API 格式（OpenAI、Qwen、QwenVision）
//! - `SseParser`: SSE (Server-Sent Events) 流式响应解析器
//! - `configs`: 各种 LLM 提供商的端点配置
//!
//! **设计原则**：
//! - 所有参数从 `AgentConfig` 和 `driver.rs` 读取，不硬编码
//! - 统一使用 `GenericHttpClient`，不再使用特定提供商的客户端
//! - 支持流式响应和普通响应

#[cfg(feature = "openai-client")]
pub mod configs;
#[cfg(feature = "openai-client")]
pub mod generic;
#[cfg(feature = "openai-client")]
pub mod stream;

#[cfg(feature = "openai-client")]
pub use configs::*;
#[cfg(feature = "openai-client")]
pub use generic::GenericHttpClient;
#[cfg(feature = "openai-client")]
pub use stream::SseParser;
