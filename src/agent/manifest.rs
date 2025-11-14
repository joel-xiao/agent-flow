use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentManifest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<AgentPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<AgentPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

impl AgentManifest {
    pub fn builder(name: impl Into<String>) -> AgentManifestBuilder {
        AgentManifestBuilder::new(name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPort {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<AgentPortSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AgentPort {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: None,
            description: None,
        }
    }

    pub fn with_schema(mut self, schema: AgentPortSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPortSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<Value>,
}

impl AgentPortSchema {
    pub fn new() -> Self {
        Self {
            type_name: None,
            format: None,
            json_schema: None,
        }
    }

    pub fn with_type(mut self, type_name: impl Into<String>) -> Self {
        self.type_name = Some(type_name.into());
        self
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn with_json_schema(mut self, schema: Value) -> Self {
        self.json_schema = Some(schema);
        self
    }
}

pub struct AgentManifestBuilder {
    manifest: AgentManifest,
}

impl AgentManifestBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            manifest: AgentManifest {
                name: name.into(),
                description: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                tools: Vec::new(),
                capabilities: Vec::new(),
            },
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.manifest.description = Some(description.into());
        self
    }

    pub fn input(mut self, port: AgentPort) -> Self {
        self.manifest.inputs.push(port);
        self
    }

    pub fn output(mut self, port: AgentPort) -> Self {
        self.manifest.outputs.push(port);
        self
    }

    pub fn tool(mut self, tool_name: impl Into<String>) -> Self {
        self.manifest.tools.push(tool_name.into());
        self
    }

    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        self.manifest.capabilities.push(capability.into());
        self
    }

    pub fn build(self) -> AgentManifest {
        self.manifest
    }
}

impl Default for AgentManifestBuilder {
    fn default() -> Self {
        Self::new("")
    }
}
