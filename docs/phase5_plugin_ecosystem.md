## Stage 5: Plugin Ecosystem, Tools & Documentation (Partially Implemented)

> **Status**: 🟡 Partially complete. Plugin mechanism and CLI tools are implemented, visualization UI is still planned.

### 1. Plugin Mechanism Design
- **Plugin Manifest**: Defines `PluginManifest` (name, version, type: Agent/Tool/Schema, dependencies, etc.), supports static configuration and hot loading.
- **Directory Specification**: `plugins/<vendor>/<plugin_name>/`, contains manifest (JSON), source or binary, schema resources.
- **Loading Mechanism**:
  - Scan plugin directory at startup, register Agent/Tool/Schema.
  - Provide `PluginRegistry` to track loaded plugins, handle version conflicts and disabling.
  - Reserve interfaces for future dynamic loading (CLI/HTTP).
- **Security/Isolation**: First phase focuses on trusted plugins; record requirements for future sandboxing/permission restrictions.

### 2. Tools & Visualization
- **CLI**:
  - `agentflow plugins list`: List plugin information and dependencies.
  - `agentflow schema export`: Export current schema/manifest.
  - `agentflow flow trace <id>`: View Flow execution logs and errors.
- **Visualization / UI** (Planned):
  - Web Dashboard displaying Flow graph, node status, tool calls, and errors.
  - Support schema/manifest browsing and search.

### 3. Documentation & Demos
- Update README with entry paths and plugin ecosystem overview.
- New documentation chapters:
  - Plugin development guide: directory structure, Manifest, registration methods.
  - Tool pipeline and structured message practice cases.
- Demo planning:
  - Multi-agent collaboration (structured messages + ToolFlow).
  - Plugin-style tool extension (enable new Tool Manifest in plugin directory).

---

### Recommended Next Implementation Order
1. Implement `PluginManifest`/`PluginRegistry` and basic directory loading (complete `phase5-plugin`).
2. Provide CLI tools (`cargo run -- plugins list`, etc.) (`phase5-tooling`).
3. Organize documentation and examples, publish stage summary (`phase5-docs`).
