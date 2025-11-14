## JSON Workflow Configuration Guide

This document describes how to define complete AgentFlow workflows using JSON configuration, including Agent, Tool, and Flow configuration.

### 1. Overview

AgentFlow supports defining workflows through JSON configuration without writing Rust code. Suitable for rapid prototyping and configuration-driven deployment.

**Environment Variable**: When using Qwen driver, set `QWEN_API_KEY`:
```bash
export QWEN_API_KEY="your-api-key-here"
```

### 2. Workflow Configuration Structure

A complete JSON workflow configuration contains three main parts:

```json
{
  "agents": [...],  // Agent configuration list
  "tools": [...],   // Tool configuration list (optional)
  "flow": {...}     // Flow graph definition
}
```

### 3. Agent Configuration

Each Agent configuration contains the following fields:

```json
{
  "name": "agent_name",           // Unique agent identifier
  "driver": "qwen",               // Driver type: qwen, echo, etc.
  "role": "Role Name",            // Role description (optional)
  "prompt": "System prompt",      // System prompt (optional)
  "model": "qwen-max",            // Model name (for LLM drivers)
  "api_key": "your-api-key",      // Optional, uses config value first, then QWEN_API_KEY env var
  "intent": "agent_intent"        // Optional
}
```

**Driver Types**: `qwen` (Qwen LLM), `echo` (local echo, for testing)

**API Key**: Recommended to provide via environment variable, omit `api_key` field in config.

### 4. Flow Graph Configuration

Flow configuration defines the workflow graph structure:

```json
{
  "name": "workflow_name",
  "start": "start_node",
  "description": "Workflow description",
  "parameters": [...],      // Input parameters (optional)
  "variables": [...],       // Variable declarations (optional)
  "nodes": [...],           // Node list
  "transitions": [...]      // Transition/edge list
}
```

#### Node Types

1. **Agent Node**
```json
{
  "kind": "agent",
  "name": "node_name",
  "agent": "agent_name"  // References agent defined in agents
}
```

2. **Decision Node**
```json
{
  "kind": "decision",
  "name": "decision_name",
  "policy": "first_match",  // or "all"
  "branches": [
    {
      "name": "branch_name",
      "condition": {
        "type": "state_equals",
        "key": "route",
        "value": "a"
      },
      "target": "target_node"
    }
  ]
}
```

3. **Join Node**
```json
{
  "kind": "join",
  "name": "join_name",
  "strategy": "all",  // or "any", "count"
  "inbound": ["node_a", "node_b"]
}
```

4. **Loop Node**
```json
{
  "kind": "loop",
  "name": "loop_name",
  "entry": "entry_node",
  "condition": {
    "type": "state_equals",
    "key": "continue",
    "value": "true"
  },
  "max_iterations": 10
}
```

5. **Tool Node**
```json
{
  "kind": "tool",
  "name": "tool_node",
  "pipeline": "pipeline_name"  // References pipeline in Tool Orchestrator
}
```

6. **Terminal Node**
```json
{
  "kind": "terminal",
  "name": "finish"
}
```

#### Transition Configuration

```json
{
  "from": "source_node",
  "to": "target_node",
  "name": "transition_name",
  "description": "Transition description",
  "condition": {...}  // Optional condition
}
```

### 5. Condition Expressions

Supports multiple condition types:

- `always`: Always true
- `state_equals`: State value equals
- `state_not_equals`: State value not equals
- `state_exists`: State key exists
- `state_absent`: State key absent

Example:
```json
{
  "type": "state_equals",
  "key": "route",
  "value": "a"
}
```

### 6. Usage

#### 6.1 Load Workflow from JSON

```rust
use agentflow::{load_workflow_from_value, WorkflowBundle};
use serde_json::json;

let config = json!({
  "agents": [...],
  "tools": [],
  "flow": {...}
});

let bundle: WorkflowBundle = load_workflow_from_value(&config)?;
let (flow, agents, tools) = bundle.into_parts();
```

#### 6.2 Execute Workflow

```rust
use agentflow::{FlowContext, FlowExecutor, MemoryStore};
use std::sync::Arc;

let ctx_store: Arc<dyn ContextStore> = Arc::new(MemoryStore::new());
let ctx = Arc::new(FlowContext::new(ctx_store));

let executor = FlowExecutor::new(flow, agents, tools);
let result = executor.start(ctx, AgentMessage::user("Initial message")).await?;
```

### 7. Examples

```bash
# Run examples (set QWEN_API_KEY first)
cargo run --example json_workflow_example --features openai-client
cargo run --example simple_chain_workflow --features openai-client
cargo run --example multi_agent_conversation --features openai-client
cargo run --example food_vision_analysis --features openai-client -- test_food.jpg
```

More examples in `examples/` and `tests/` directories.

### 8. Image Input

Vision models (e.g., `qwen3-vl-plus`) support the following ways to pass images:

```json
{
  "image_base64": "base64_encoded_data",  // Base64 encoded
  "image_path": "path/to/image.jpg",      // Local path
  "image_url": "https://example.com/img.jpg"  // URL
}
```

Image data is automatically passed between agents.

### 9. Streaming Output

LLM-driven agents support streaming output, displaying response content in real-time.

### 10. Best Practices

- Use meaningful names and clear prompts
- Provide API keys via environment variables, do not hardcode
- Use `echo` driver for local testing
- Use key management services in production

### 11. Limitations

- Custom Rust Agent implementations not supported
- Complex logic recommended to use programmatic FlowBuilder
- Must provide API key via `QWEN_API_KEY` environment variable

### 12. Error Handling

All errors are directly returned without fallback handling. Ensure proper error handling in your code:

```rust
match executor.start(ctx, message).await {
    Ok(result) => println!("Success: {:?}", result),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

### 13. Multi-Agent Conversations

Support for multi-role, multi-agent conversations through JSON configuration. Each agent can have different roles and prompts, enabling collaborative workflows.

Example:
```json
{
  "agents": [
    {
      "name": "product_manager",
      "driver": "qwen",
      "role": "Product Manager",
      "prompt": "You are an experienced product manager...",
      "model": "qwen-max"
    },
    {
      "name": "tech_expert",
      "driver": "qwen",
      "role": "Technical Expert",
      "prompt": "You are a senior technical expert...",
      "model": "qwen-max"
    }
  ],
  "flow": {
    "transitions": [
      {"from": "product_manager", "to": "tech_expert"}
    ]
  }
}
```
