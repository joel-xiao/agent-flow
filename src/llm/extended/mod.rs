//! 扩展 LLM 模块
//!
//! 此模块提供JSON配置驱动的API客户端功能：
//! - `JsonApiClient`: 基于JSON配置的API客户端
//! - `UniversalApiClient`: 通用 API 客户端构建器
//! - JSON 配置支持
//! - 服务配置管理
//!
//! **重要**: 
//! - 核心LLM调用请使用 `crate::llm::http::GenericHttpClient`
//! - 此模块主要用于需要JSON配置驱动的高级用例
//!
//! **v2.0 变更**:
//! - 已移除 `GenericApiClient` 和 `ExtendedApiClient`（使用 `GenericHttpClient` 替代）
//! - 已移除 client 目录（包含大量厂商专用代码）
//! - 保留JSON配置相关功能用于特殊场景

pub mod json_config;
pub mod json_unified;
pub mod service_config;
pub mod universal;

pub use json_config::{ApiCallRequest, EndpointConfig, JsonApiClient, JsonApiConfig};
pub use json_unified::{
    ApiCallConfig as UnifiedApiCallConfig, ApiProviderConfig, EndpointDefinition,
    UnifiedApiManager, UnifiedJsonConfig,
};
pub use service_config::{
    ApiEdge, ApiGraph, ApiNode, ServiceConfig, ServiceDefinition, ServiceManager,
};
pub use universal::{ApiBuilder, ApiCallBuilder, UniversalApiClient};
