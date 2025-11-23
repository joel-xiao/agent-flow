// Schema 模块

mod error;
mod schema;
mod registry;
mod validation;

pub use error::SchemaError;
pub use schema::{Schema, SchemaKind};
pub use registry::SchemaRegistry;

use std::sync::{Mutex, OnceLock};
use serde_json::Value;

static REGISTRY: OnceLock<Mutex<SchemaRegistry>> = OnceLock::new();

/// 获取全局 Schema 注册表
pub fn registry() -> &'static Mutex<SchemaRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(SchemaRegistry::new()))
}

/// 注册 Schema
pub fn register_schema(name: impl Into<String>, schema: Schema) {
    if let Ok(mut guard) = registry().lock() {
        guard.register(name, schema);
    } else {
        tracing::warn!("failed to acquire schema registry lock");
    }
}

/// 验证值是否符合指定的 Schema
pub fn validate_schema(name: &str, value: &Value) -> std::result::Result<(), SchemaError> {
    if let Ok(guard) = registry().lock() {
        guard.validate(name, value)
    } else {
        Err(SchemaError::RegistryPoisoned)
    }
}

/// 获取所有 Schema 的快照
pub fn schemas_snapshot() -> Vec<(String, Schema)> {
    if let Ok(guard) = registry().lock() {
        guard.snapshot()
    } else {
        Vec::new()
    }
}
