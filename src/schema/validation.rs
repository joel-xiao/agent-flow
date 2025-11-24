use super::error::SchemaError;
use super::schema::{Schema, SchemaKind};

/// 验证值是否符合 Schema
pub fn validate_value(
    schema: &Schema,
    value: &serde_json::Value,
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
