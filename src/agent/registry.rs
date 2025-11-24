use std::collections::HashMap;
use std::sync::Arc;



use super::agent::Agent;

pub type AgentRegistry = HashMap<String, Arc<dyn Agent>>;

pub fn register_agent(name: &str, agent: Arc<dyn Agent>, registry: &mut AgentRegistry) {
    registry.insert(name.to_string(), agent);
}
