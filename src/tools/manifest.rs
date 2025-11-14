use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolManifest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<ToolPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<ToolPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<String>,
}

impl ToolManifest {
    pub fn builder(name: impl Into<String>) -> ToolManifestBuilder {
        ToolManifestBuilder::new(name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolPort {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<ToolPortSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,
}

impl ToolPort {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: None,
            description: None,
            example: None,
        }
    }

    pub fn with_schema(mut self, schema: ToolPortSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_example(mut self, example: Value) -> Self {
        self.example = Some(example);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolPortSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<Value>,
}

impl ToolPortSchema {
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

#[derive(Clone, Debug)]
pub struct ToolManifestBuilder {
    manifest: ToolManifest,
}

impl ToolManifestBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            manifest: ToolManifest {
                name: name.into(),
                description: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                capabilities: Vec::new(),
                permissions: Vec::new(),
                resources: Vec::new(),
            },
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.manifest.description = Some(description.into());
        self
    }

    pub fn input(mut self, port: ToolPort) -> Self {
        self.manifest.inputs.push(port);
        self
    }

    pub fn output(mut self, port: ToolPort) -> Self {
        self.manifest.outputs.push(port);
        self
    }

    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        self.manifest.capabilities.push(capability.into());
        self
    }

    pub fn permission(mut self, permission: impl Into<String>) -> Self {
        self.manifest.permissions.push(permission.into());
        self
    }

    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.manifest.resources.push(resource.into());
        self
    }

    pub fn build(self) -> ToolManifest {
        self.manifest
    }
}

impl Default for ToolManifestBuilder {
    fn default() -> Self {
        Self::new("")
    }
}
