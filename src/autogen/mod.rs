pub mod yaml;

use crate::agent::{AgentFactoryRegistry, AgentRegistry};
use crate::flow::{Flow, FlowRegistry};
use crate::tools::{ToolFactoryRegistry, ToolRegistry};

pub use yaml::{
    AgentSpec, AutogenConfig, FlowSpec, NodeSpec, ToolSpec, TransitionSpec, load_flows_from_file,
    load_flows_from_str,
};

pub trait AutogenProvider {
    fn register_agents(&self, _registry: &mut AgentRegistry) {}
    fn register_tools(&self, _registry: &mut ToolRegistry) {}
    fn register_flow(&self) -> Vec<Flow> {
        Vec::new()
    }
    fn register_all(
        &self,
        _agent_factories: &AgentFactoryRegistry,
        agent_registry: &mut AgentRegistry,
        _tool_factories: &ToolFactoryRegistry,
        tool_registry: &mut ToolRegistry,
        flow_registry: &mut FlowRegistry,
    ) -> crate::error::Result<()> {
        self.register_agents(agent_registry);
        self.register_tools(tool_registry);
        for flow in self.register_flow() {
            flow_registry.register(flow);
        }
        Ok(())
    }
}
