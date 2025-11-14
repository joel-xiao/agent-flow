use std::path::Path;

use serde::Serialize;

use crate::plugin::{PluginError, PluginManifest, PluginRegistry};
use crate::schema::{Schema, schemas_snapshot};

#[derive(Clone, Debug, Serialize)]
pub struct SchemaExportEntry {
    pub name: String,
    pub schema: Schema,
}

pub fn load_plugin_manifests(dir: &Path) -> Result<Vec<PluginManifest>, PluginError> {
    let mut registry = PluginRegistry::new();
    registry.load_directory(dir)?;
    Ok(registry.manifests().cloned().collect())
}

pub fn schema_exports() -> Vec<SchemaExportEntry> {
    schemas_snapshot()
        .into_iter()
        .map(|(name, schema)| SchemaExportEntry { name, schema })
        .collect()
}
