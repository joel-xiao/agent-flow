//! 扩展 LLM 客户端模块
//!
//! 此模块提供扩展的 LLM API 客户端功能，包括：
//! - `GenericApiClient`: 通用 API 客户端
//! - `UniversalApiClient`: 通用 API 客户端构建器
//! - `ExtendedApiClient`: 扩展 API 客户端 trait
//! - JSON 配置支持
//! - 服务配置管理
//!
//! **注意**：此模块主要用于高级用例和扩展功能。
//! 对于大多数用例，建议使用 `crate::llm::http::GenericHttpClient`。
//!
//! **状态**：此模块目前处于维护状态，新功能建议优先考虑使用核心模块。

pub mod traits;
pub mod types;
pub mod client;
pub mod universal;
pub mod json_config;
pub mod json_unified;
pub mod service_config;
#[cfg(test)]
pub mod examples;

pub use traits::{ExtendedApiClient, DynExtendedApiClient};
pub use types::*;
pub use client::GenericApiClient;
pub use universal::{UniversalApiClient, ApiBuilder, ApiCallBuilder};
pub use json_config::{JsonApiConfig, JsonApiClient, ApiCallRequest, EndpointConfig};
pub use json_unified::{UnifiedJsonConfig, UnifiedApiManager, ApiProviderConfig, ApiCallConfig as UnifiedApiCallConfig, EndpointDefinition};
pub use service_config::{ServiceConfig, ServiceManager, ServiceDefinition, ApiGraph, ApiNode, ApiEdge};

