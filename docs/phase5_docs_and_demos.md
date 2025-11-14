## Stage 5 Documentation & Demo Plan

This document summarizes usage guidelines for the enhanced AgentFlow and provides demo entry points to help teams quickly understand the new architecture.

### 1. Quick Start Index
- **JSON Workflow Configuration**: See `docs/json_workflow_configuration.md` for how to configure complete workflows using JSON, including Agent, Tool, and Flow definitions.
- **Flow DSL Extension**: See `docs/phase1_flow_extension.md` for `DecisionNode`, `JoinNode`, `LoopNode`, and `FlowContext`.
- **Agent Capability Declarations**: See `docs/phase2_agent_manifest.md` for `AgentManifest`, typed I/O, and lifecycle hooks.
- **Tool System Evolution**: See `docs/phase3_tool_extension.md` for Tool Manifest, Orchestrator, and resource management.
- **Unified Schema/Message/Error**: See `docs/phase4_schema_and_errors.md` for structured messages and error handling.
- **Plugin Ecosystem Planning**: See `docs/phase5_plugin_ecosystem.md` for plugin directory specifications and Registry design.

### 2. CLI Usage Guide
Run `cargo run --bin agentflow -- <command>` or execute `agentflow` directly after installation.

#### 2.1 Plugin Commands
- `agentflow plugins list --dir <path>`  
  - Scans plugin directory (default `plugins/`), displays name, version, type, and description from `plugin.json`.
  - Recommended directory structure: `plugins/<vendor>/<plugin_name>/plugin.json`.

#### 2.2 Schema Commands
- `agentflow schema export --format json --pretty`  
  - Exports currently registered Schema list to stdout.
- `agentflow schema export --format json --output ./schema_dump.json`  
  - Supports JSON format, `--pretty` controls JSON formatting.

#### 2.3 Flow Debugging (Placeholder)
- `agentflow flow trace <id>`  
  - Currently returns a message; will output actual execution trace after Flow trace persistence is implemented.

### 3. Demo Scenarios
1. **JSON Configuration Multi-Agent Collaboration**  
   - Use JSON to define multi-role conversation flows (see `tests/json_config_flow_tests.rs::json_config_multi_agent_conversation`).
   - Demonstrates how to implement product manager, technical expert, designer, and other role collaboration through JSON configuration.
   - Shows streaming output functionality, displaying each role's conversation content in real-time.
2. **Vision Analysis Workflow**  
   - Use JSON to define food vision analysis flow (see `tests/food_vision_analysis_tests.rs::food_vision_multi_agent_conversation`).
   - Shows multi-expert collaboration: food identification, portion estimation, nutrition calculation, health advice, exercise advice, diet planning.
   - Demonstrates image input (base64/URL/path) and vision model integration.
3. **Tool Pipeline + StructuredMessage**  
   - Write Tool Pipeline with `Sequential` / `Fallback` strategies, using `StructuredMessage<T>` to serialize complex results.
   - Demonstrates error messages when Schema validation fails.
4. **Plugin Extension Example**  
   - Declare new Tool in `plugins/acme/image_tool/plugin.json` and prepare corresponding implementation.
   - Register using `PluginRegistry::load_directory` at startup, view plugin info via CLI.

### 4. Documentation TODO
- ✅ Plugin ecosystem planning (`phase5_plugin_ecosystem.md`).
- ✅ CLI usage guide (this document).
- ✅ Agent Manifest in-depth guide (`docs/phase2_agent_manifest.md`).
- ✅ JSON workflow configuration guide (`docs/json_workflow_configuration.md`).
- ✅ Streaming output feature description (included in JSON configuration guide).
- ☐ Demo tutorial: structured messages + Tool Pipeline, plugin-style Tool examples.
- ☐ CLI scenario screenshots or recordings (place in docs/assets when complete).

### 5. Pre-Release Checklist
- [ ] Add index to README (or main site) pointing to the above documentation.
- [ ] Add example repository or scripts for quick demo runs.
- [ ] Configure `scripts/regression.sh` to run CLI Smoke Test (e.g., export Schema).

---

This document will be updated in real-time as Stage 5 is implemented. Check off TODOs after completing each subtask, and attach actual example links or commands in the Demo section.
