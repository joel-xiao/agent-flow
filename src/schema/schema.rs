use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema 类型枚举
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

/// Schema 定义
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
