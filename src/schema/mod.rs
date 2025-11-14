use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::error::{ErrorSeverity, FrameworkError};

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("schema `{0}` not registered")]
    NotRegistered(String),
    #[error("schema validation failed: {message}")]
    Validation { message: String, path: Vec<String> },
    #[error("schema registry lock poisoned")]
    RegistryPoisoned,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SchemaKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "array")]
    Array { items: Box<Schema> },
    #[serde(rename = "object")]
    Object {
        properties: HashMap<String, Schema>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        required: Vec<String>,
        #[serde(default = "Schema::allow_additional")]
        additional: bool,
    },
    #[serde(rename = "any")]
    Any,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(flatten)]
    pub kind: SchemaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Schema {
    pub fn new(kind: SchemaKind) -> Self {
        Self {
            name: None,
            kind,
            description: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn allow_additional() -> bool {
        true
    }
}

#[derive(Default)]
pub struct SchemaRegistry {
    schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, schema: Schema) {
        self.schemas.insert(name.into(), schema);
    }

    pub fn snapshot(&self) -> Vec<(String, Schema)> {
        self.schemas
            .iter()
            .map(|(name, schema)| (name.clone(), schema.clone()))
            .collect()
    }

    pub fn get(&self, name: &str) -> std::result::Result<&Schema, SchemaError> {
        self.schemas
            .get(name)
            .ok_or_else(|| SchemaError::NotRegistered(name.to_string()))
    }

    pub fn validate(&self, name: &str, value: &Value) -> std::result::Result<(), SchemaError> {
        let schema = self.get(name)?;
        validate_value(schema, value, &mut Vec::new())
    }
}

fn validate_value(
    schema: &Schema,
    value: &Value,
    path: &mut Vec<String>,
) -> std::result::Result<(), SchemaError> {
    match &schema.kind {
        SchemaKind::Null => {
            if !value.is_null() {
                return Err(SchemaError::Validation {
                    message: "expected null".to_string(),
                    path: path.clone(),
                });
            }
        }
        SchemaKind::Boolean => {
            if !value.is_boolean() {
                return Err(SchemaError::Validation {
                    message: "expected boolean".to_string(),
                    path: path.clone(),
                });
            }
        }
        SchemaKind::Integer => {
            if !value.is_i64() {
                return Err(SchemaError::Validation {
                    message: "expected integer".to_string(),
                    path: path.clone(),
                });
            }
        }
        SchemaKind::Number => {
            if !value.is_number() {
                return Err(SchemaError::Validation {
                    message: "expected number".to_string(),
                    path: path.clone(),
                });
            }
        }
        SchemaKind::String => {
            if !value.is_string() {
                return Err(SchemaError::Validation {
                    message: "expected string".to_string(),
                    path: path.clone(),
                });
            }
        }
        SchemaKind::Array { items } => {
            if let Some(array) = value.as_array() {
                for (idx, element) in array.iter().enumerate() {
                    path.push(idx.to_string());
                    validate_value(items, element, path)?;
                    path.pop();
                }
            } else {
                return Err(SchemaError::Validation {
                    message: "expected array".to_string(),
                    path: path.clone(),
                });
            }
        }
        SchemaKind::Object {
            properties,
            required,
            additional,
        } => {
            let object = value.as_object().ok_or_else(|| SchemaError::Validation {
                message: "expected object".to_string(),
                path: path.clone(),
            })?;

                for key in required {
                if !object.contains_key(key) {
                    let mut required_path = path.clone();
                    required_path.push(key.clone());
                    return Err(SchemaError::Validation {
                        message: format!("missing required property `{}`", key),
                        path: required_path,
                    });
                    }
                }

            for (key, val) in object {
                if let Some(sub_schema) = properties.get(key) {
                    path.push(key.clone());
                    validate_value(sub_schema, val, path)?;
                    path.pop();
                } else if !additional {
                    let mut extra_path = path.clone();
                    extra_path.push(key.clone());
                    return Err(SchemaError::Validation {
                        message: format!("unexpected property `{}`", key),
                        path: extra_path,
                    });
                    }
                }
        }
        SchemaKind::Any => {}
    }

                Ok(())
}

static REGISTRY: OnceLock<Mutex<SchemaRegistry>> = OnceLock::new();

pub fn registry() -> &'static Mutex<SchemaRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(SchemaRegistry::new()))
}

pub fn register_schema(name: impl Into<String>, schema: Schema) {
    if let Ok(mut guard) = registry().lock() {
        guard.register(name, schema);
    } else {
        tracing::warn!("failed to acquire schema registry lock");
    }
}

pub fn validate_schema(name: &str, value: &Value) -> std::result::Result<(), SchemaError> {
    if let Ok(guard) = registry().lock() {
        guard.validate(name, value)
    } else {
        Err(SchemaError::RegistryPoisoned)
    }
}

pub fn schemas_snapshot() -> Vec<(String, Schema)> {
    if let Ok(guard) = registry().lock() {
        guard.snapshot()
            } else {
        Vec::new()
    }
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
