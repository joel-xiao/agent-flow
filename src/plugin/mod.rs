use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::error::AgentFlowError;
use crate::schema::{register_schema, Schema, SchemaKind};
use crate::tools::ToolRegistry;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Agent,
    Tool,
    Schema,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default = "PluginManifest::default_kind")]
    pub kind: PluginKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schemas: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

impl PluginManifest {
    fn default_kind() -> PluginKind {
        PluginKind::Other
    }
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin manifest not found: {0}")]
    ManifestMissing(String),
    #[error("failed to parse plugin manifest: {0}")]
    ManifestParse(String),
    #[error("plugin `{name}` incompatible: {reason}")]
    Incompatible { name: String, reason: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct PluginRegistry {
    manifests: HashMap<String, PluginManifest>,
    base_dir: Option<PathBuf>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
            base_dir: None,
        }
    }

    pub fn with_base_dir(mut self, base: PathBuf) -> Self {
        self.base_dir = Some(base);
        self
    }

    pub fn load_directory(&mut self, dir: impl AsRef<Path>) -> Result<(), PluginError> {
        let dir = dir.as_ref();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    let manifest = Self::load_manifest(&manifest_path)?;
                    self.register_manifest(manifest);
                }
            }
        }
        Ok(())
    }

    pub fn register_manifest(&mut self, manifest: PluginManifest) {
        self.manifests.insert(manifest.name.clone(), manifest);
    }

    pub fn manifests(&self) -> impl Iterator<Item = &PluginManifest> {
        self.manifests.values()
    }

    fn load_manifest(path: &Path) -> Result<PluginManifest, PluginError> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|err| PluginError::ManifestParse(err.to_string()))
    }

    pub fn initialize(&self, tools: &mut ToolRegistry) -> Result<(), AgentFlowError> {
        for manifest in self.manifests.values() {
            match manifest.kind {
                PluginKind::Tool => self.initialize_tools(manifest, tools)?,
                PluginKind::Schema => self.initialize_schemas(manifest)?,
                PluginKind::Agent | PluginKind::Other => {
                    tracing::debug!(plugin = %manifest.name, kind = ?manifest.kind, "plugin loaded");
                }
            }
        }
        Ok(())
    }

    fn initialize_tools(
        &self,
        manifest: &PluginManifest,
        _tools: &mut ToolRegistry,
    ) -> Result<(), AgentFlowError> {
        tracing::debug!(plugin = %manifest.name, tools = ?manifest.tools, "tool plugin registered");
        Ok(())
    }

    fn initialize_schemas(&self, manifest: &PluginManifest) -> Result<(), AgentFlowError> {
        for schema_name in &manifest.schemas {
            tracing::debug!(plugin = %manifest.name, schema = schema_name, "schema plugin registered");
            register_schema(
                schema_name.clone(),
                Schema::new(SchemaKind::Any).with_name(schema_name.clone()),
            );
        }
        Ok(())
    }
}
