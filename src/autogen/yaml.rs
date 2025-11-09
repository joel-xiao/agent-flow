use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::agent::{AgentFactoryRegistry, AgentRegistry};
use crate::error::{AgentFlowError, Result};
use crate::flow::{
    Flow, FlowBuilder, FlowRegistry, TransitionCondition, condition_always, condition_state_absent,
    condition_state_equals, condition_state_exists, condition_state_not_equals,
};
use crate::tools::{ToolFactoryRegistry, ToolRegistry};

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct AutogenConfig {
    #[serde(default)]
    pub flows: Vec<FlowSpec>,
    #[serde(default)]
    pub agents: Vec<AgentSpec>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FlowSpec {
    pub name: String,
    #[serde(default)]
    pub start: Option<String>,
    pub nodes: Vec<NodeSpec>,
    #[serde(default)]
    pub transitions: Vec<TransitionSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NodeSpec {
    pub name: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub terminal: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionSpec {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<ConditionSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentSpec {
    pub name: String,
    pub factory: String,
    #[serde(default)]
    pub config: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolSpec {
    pub factory: String,
    #[serde(default)]
    pub config: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConditionSpec {
    Always,
    Equals { key: String, value: String },
    NotEquals { key: String, value: String },
    Exists { key: String },
    Absent { key: String },
}

impl ConditionSpec {
    fn to_condition(&self) -> TransitionCondition {
        match self {
            ConditionSpec::Always => condition_always(),
            ConditionSpec::Equals { key, value } => {
                condition_state_equals(key.clone(), value.clone())
            }
            ConditionSpec::NotEquals { key, value } => {
                condition_state_not_equals(key.clone(), value.clone())
            }
            ConditionSpec::Exists { key } => condition_state_exists(key.clone()),
            ConditionSpec::Absent { key } => condition_state_absent(key.clone()),
        }
    }
}

impl FlowSpec {
    pub fn build(&self) -> Result<Flow> {
        let mut builder = FlowBuilder::new(&self.name);
        if let Some(start) = &self.start {
            builder.set_start(start);
        }

        for node in &self.nodes {
            if node.terminal {
                builder.add_terminal_node(&node.name);
            } else if let Some(agent) = &node.agent {
                builder.add_agent_node(&node.name, agent);
            } else {
                return Err(AgentFlowError::Other(anyhow::anyhow!(
                    "节点 `{}` 必须指定 agent 或标记 terminal",
                    node.name
                )));
            }
        }

        for transition in &self.transitions {
            if let Some(condition) = transition.condition.as_ref() {
                builder.connect_conditional_named(
                    &transition.from,
                    &transition.to,
                    transition.name.clone(),
                    condition.to_condition(),
                );
            } else {
                builder.connect_named(&transition.from, &transition.to, transition.name.clone());
            }
        }

        Ok(builder.build())
    }
}

impl AutogenConfig {
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = String::new();
        reader
            .read_to_string(&mut buf)
            .map_err(|e| AgentFlowError::Other(e.into()))?;
        let config: AutogenConfig =
            serde_yaml::from_str(&buf).map_err(|e| AgentFlowError::Other(e.into()))?;
        Ok(config)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(|e| AgentFlowError::Other(e.into()))?;
        Self::from_reader(file)
    }

    pub fn register_flows_into(&self, registry: &mut FlowRegistry) -> Result<()> {
        for flow in &self.flows {
            registry.register(flow.build()?);
        }
        Ok(())
    }

    pub fn register_agents_into(
        &self,
        factories: &AgentFactoryRegistry,
        registry: &mut AgentRegistry,
    ) -> Result<()> {
        for agent in &self.agents {
            let instance = factories.build(&agent.factory, agent.config.clone())?;
            registry.insert(agent.name.clone(), instance);
        }
        Ok(())
    }

    pub fn register_tools_into(
        &self,
        factories: &ToolFactoryRegistry,
        registry: &mut ToolRegistry,
    ) -> Result<()> {
        for tool in &self.tools {
            let instance = factories.build(&tool.factory, tool.config.clone())?;
            registry.register(instance);
        }
        Ok(())
    }

    pub fn register_all(
        &self,
        agent_factories: &AgentFactoryRegistry,
        agent_registry: &mut AgentRegistry,
        tool_factories: &ToolFactoryRegistry,
        tool_registry: &mut ToolRegistry,
        flow_registry: &mut FlowRegistry,
    ) -> Result<()> {
        self.register_agents_into(agent_factories, agent_registry)?;
        self.register_tools_into(tool_factories, tool_registry)?;
        self.register_flows_into(flow_registry)?;
        Ok(())
    }
}

pub fn load_flows_from_str(contents: &str) -> Result<Vec<Flow>> {
    let config: AutogenConfig =
        serde_yaml::from_str(contents).map_err(|e| AgentFlowError::Other(e.into()))?;
    config
        .flows
        .iter()
        .map(|spec| spec.build())
        .collect::<Result<Vec<_>>>()
}

pub fn load_flows_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Flow>> {
    let config = AutogenConfig::from_path(path)?;
    config
        .flows
        .iter()
        .map(|spec| spec.build())
        .collect::<Result<Vec<_>>>()
}
