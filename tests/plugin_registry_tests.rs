use std::fs;

use agentflow::{
    PluginKind, PluginManifest, PluginRegistry, Schema, SchemaKind, ToolRegistry,
    load_plugin_manifests, register_schema, schema_exports,
};
use tempfile::tempdir;

fn sample_manifest(kind: PluginKind) -> PluginManifest {
    PluginManifest {
        name: "sample.plugin".to_string(),
        version: "0.1.0".to_string(),
        kind,
        description: None,
        agents: Vec::new(),
        tools: vec!["tool-a".to_string()],
        schemas: vec!["schema-a".to_string()],
        metadata: None,
        dependencies: Vec::new(),
    }
}

#[test]
fn registry_registers_manifest() {
    let mut registry = PluginRegistry::new();
    registry.register_manifest(sample_manifest(PluginKind::Tool));
    let manifests: Vec<_> = registry.manifests().collect();
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].name, "sample.plugin");
}

#[test]
fn registry_initializes_without_error() {
    let mut registry = PluginRegistry::new();
    registry.register_manifest(sample_manifest(PluginKind::Schema));

    let mut tool_registry = ToolRegistry::new();
    let result = registry.initialize(&mut tool_registry);
    assert!(result.is_ok());
}

#[test]
fn load_plugin_manifests_from_directory() {
    let dir = tempdir().expect("temp dir");
    let plugin_dir = dir.path().join("demo_plugin");
    fs::create_dir(&plugin_dir).expect("create plugin dir");
    let manifest = serde_json::json!({
        "name": "demo.plugin",
        "version": "0.2.0",
        "kind": "tool",
        "description": "demo plugin"
    });
    fs::write(plugin_dir.join("plugin.json"), manifest.to_string()).expect("write manifest");

    let manifests = load_plugin_manifests(dir.path()).expect("load manifests");
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].name, "demo.plugin");
}

#[test]
fn schema_exports_include_registered_schema() {
    let name = format!(
        "cli.test.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_millis()
    );
    register_schema(&name, Schema::new(SchemaKind::String).with_name(&name));

    let exports = schema_exports();
    assert!(
        exports.iter().any(|entry| entry.name == name),
        "exports should include newly registered schema"
    );
}
