# AgentFlow Documentation Index

This directory contains the complete documentation for the AgentFlow framework.

## Quick Start

- **[JSON Workflow Configuration Guide](./json_workflow_configuration.md)** ⭐ Recommended
  - Complete JSON workflow configuration
  - Agent, Tool, and Flow configuration
  - Streaming output and image input

## Architecture & Design

- **[Stage 0 Overview](./enhancement_stage0_overview.md)**
  - Project scope and boundaries
  - Existing architecture summary
  - Regression testing strategy

- **[Stage 1: Flow DSL Extension](./phase1_flow_extension.md)** ✅ Implemented
  - Decision, Join, Loop nodes
  - FlowContext extension
  - Parameters and variables management
  - JSON configuration support

- **[Stage 2: Agent Manifest](./phase2_agent_manifest.md)** ✅ Implemented
  - Agent capability declarations
  - Typed I/O
  - Lifecycle hooks
  - Context access API

- **[Stage 3: Tool System Extension](./phase3_tool_extension.md)** ✅ Implemented
  - Tool Manifest
  - Tool Orchestrator
  - Resource management (ModelRegistry, ToolResourceManager)
  - Flow/Agent integration

- **[Stage 4: Schema & Error Handling](./phase4_schema_and_errors.md)** ✅ Implemented
  - Unified Schema management
  - Structured message encapsulation
  - Unified error handling mechanism

- **[Stage 5: Plugin Ecosystem](./phase5_plugin_ecosystem.md)** 🟡 Partially Implemented
  - Plugin mechanism design
  - Tools and visualization (CLI implemented)
  - Documentation and Demos

- **[Stage 5: Documentation & Demos](./phase5_docs_and_demos.md)**
  - CLI usage guide
  - Demo scenarios
  - Documentation TODO list

## Usage Guides

- [JSON Workflow Configuration](./json_workflow_configuration.md) - JSON configuration method
- Stage documentation - FlowBuilder, Agent Manifest, Tool Orchestrator, Schema API

## CLI Tools

AgentFlow provides command-line tools for managing plugins and exporting schemas:

```bash
# List plugins
agentflow plugins list

# Export schema
agentflow schema export --format json --pretty

# Flow debugging (planned)
agentflow flow trace <id>
```

For details, see [Stage 5 Documentation](./phase5_docs_and_demos.md#2-cli-usage-guide).

## Test Examples

- `tests/json_config_flow_tests.rs` - JSON configuration (decision, loop, join nodes)
- `tests/food_vision_analysis_tests.rs` - Vision analysis (multi-expert collaboration)
- `tests/complex_diet_masterplan_tests.rs` - Complex flow (full feature stack)

## Quick Start

**Environment Variable**: `export QWEN_API_KEY="your-api-key-here"`

**Run Examples**:
```bash
cargo run --example json_workflow_example --features openai-client
cargo run --example simple_chain_workflow --features openai-client
cargo run --example multi_agent_conversation --features openai-client
cargo run --example food_vision_analysis --features openai-client -- test_food.jpg
```

**Run Tests**: `QWEN_API_KEY="your-key" cargo test --features openai-client`

### Code Example

```rust
use agentflow::{load_workflow_from_value, FlowContext, FlowExecutor, MemoryStore};
use agentflow::agent::AgentMessage;
use serde_json::json;
use std::sync::Arc;

let config = json!({
    "agents": [{
        "name": "assistant",
        "driver": "qwen",
        "role": "Assistant",
        "model": "qwen-max"
    }],
    "flow": {
        "name": "simple_flow",
        "start": "assistant",
        "nodes": [
            {"kind": "agent", "name": "assistant", "agent": "assistant"},
            {"kind": "terminal", "name": "finish"}
        ],
        "transitions": [{"from": "assistant", "to": "finish"}]
    }
});

let bundle = load_workflow_from_value(&config)?;
let (flow, agents, tools) = bundle.into_parts();
let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
let executor = FlowExecutor::new(flow, agents, tools);
let result = executor.start(ctx, AgentMessage::user("Hello")).await?;
```

## Documentation Status

- ✅ Implemented and completed
- 🟡 Partially implemented
- ⏳ Planned

## Related Resources

- Source code: `src/` directory
- Test examples: `tests/` directory
- CLI tool: `src/bin/agentflow.rs`
