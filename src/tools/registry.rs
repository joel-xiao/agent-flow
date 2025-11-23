use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::AgentMessage;
use crate::error::{AgentFlowError, Result};
use crate::state::FlowContext;
use crate::tools::manifest::ToolManifest;
use crate::tools::tool::Tool;

#[derive(Clone)]
struct ToolEntry {
    tool: Arc<dyn Tool>,
    manifest: Option<Arc<ToolManifest>>,
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let _ = self.insert(tool, None);
    }

    pub fn register_with_manifest(
        &mut self,
        tool: Arc<dyn Tool>,
        manifest: ToolManifest,
    ) -> Result<()> {
        self.insert(tool, Some(manifest))
    }

    pub fn register_manifest(&mut self, manifest: ToolManifest) -> Result<()> {
        let entry = self
            .tools
            .get_mut(&manifest.name)
            .ok_or_else(|| AgentFlowError::ToolNotRegistered(manifest.name.clone()))?;
        entry.manifest = Some(Arc::new(manifest));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|entry| Arc::clone(&entry.tool))
    }

    pub fn manifest(&self, name: &str) -> Option<Arc<ToolManifest>> {
        self.tools
            .get(name)
            .and_then(|entry| entry.manifest.as_ref().map(Arc::clone))
    }

    fn insert(&mut self, tool: Arc<dyn Tool>, manifest: Option<ToolManifest>) -> Result<()> {
        if let Some(ref manifest) = manifest {
            if manifest.name != tool.name() {
                return Err(AgentFlowError::ManifestMismatch {
                    kind: "tool",
                    name: tool.name().to_string(),
                });
            }
        }

        self.tools.insert(
            tool.name().to_string(),
            ToolEntry {
                tool,
                manifest: manifest.map(Arc::new),
            },
        );
        Ok(())
    }
}

