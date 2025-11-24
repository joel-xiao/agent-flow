use crate::error::{ErrorSeverity, FrameworkError};
use serde_json::json;
use thiserror::Error;

/// Schema 错误类型
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("schema `{0}` not registered")]
    NotRegistered(String),
    #[error("schema validation failed: {message}")]
    Validation { message: String, path: Vec<String> },
    #[error("schema registry lock poisoned")]
    RegistryPoisoned,
}

impl From<SchemaError> for FrameworkError {
    fn from(error: SchemaError) -> Self {
        match error {
            SchemaError::NotRegistered(name) => FrameworkError::new(
                "schema.not_registered",
                format!("schema `{name}` not registered"),
            )
            .with_severity(ErrorSeverity::Warning),
            SchemaError::Validation { message, path } => {
                FrameworkError::new("schema.validation_failed", message)
                    .with_severity(ErrorSeverity::Warning)
                    .with_context(json!({ "path": path }))
            }
            SchemaError::RegistryPoisoned => {
                FrameworkError::new("schema.registry_poisoned", "schema registry lock poisoned")
                    .with_severity(ErrorSeverity::Critical)
            }
        }
    }
}
