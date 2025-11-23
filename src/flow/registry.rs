use std::collections::HashMap;
use crate::flow::types::Flow;

/// Flow 注册表
#[derive(Default)]
pub struct FlowRegistry {
    flows: HashMap<String, Flow>,
}

impl FlowRegistry {
    pub fn new() -> Self {
        Self {
            flows: HashMap::new(),
        }
    }

    pub fn register(&mut self, flow: Flow) {
        self.flows.insert(flow.name.clone(), flow);
    }

    pub fn get(&self, name: &str) -> Option<&Flow> {
        self.flows.get(name)
    }

    pub fn list(&self) -> impl Iterator<Item = &Flow> {
        self.flows.values()
    }
}

