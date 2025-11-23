use std::collections::HashMap;
use super::schema::Schema;
use super::error::SchemaError;

/// Schema 注册表
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

    pub fn validate(&self, name: &str, value: &serde_json::Value) -> std::result::Result<(), SchemaError> {
        let schema = self.get(name)?;
        super::validation::validate_value(schema, value, &mut Vec::new())
    }
}

