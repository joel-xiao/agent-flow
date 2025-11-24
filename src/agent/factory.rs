use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::error::{AgentFlowError, Result};

use super::agent::Agent;

pub type AgentFactory = Arc<dyn Fn(Option<Value>) -> Result<Arc<dyn Agent>> + Send + Sync>;

#[derive(Default)]
pub struct AgentFactoryRegistry {
    factories: HashMap<String, AgentFactory>,
}

impl AgentFactoryRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register_factory<T: Into<String>>(&mut self, name: T, factory: AgentFactory) {
        self.factories.insert(name.into(), factory);
    }

    pub fn build(&self, factory_name: &str, config: Option<Value>) -> Result<Arc<dyn Agent>> {
        let builder = self
            .factories
            .get(factory_name)
            .ok_or_else(|| AgentFlowError::AgentNotRegistered(factory_name.to_string()))?;
        builder(config)
    }

    pub fn has_factory(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }
}
