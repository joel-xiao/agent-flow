## AgentFlow Enhancement Stage 0 Overview

### Scope & Boundaries
- Focus on core framework: `flow`, `runtime`, `agent`, `tools`, `state`, and supporting registries.
- No changes to business-specific agents or downstream consumers in this iteration.
- Maintain current public APIs as compatible while we prepare extension points.

### Existing Architecture Summary
- **Flow Graph**: `Flow`, `FlowBuilder`, and `FlowNodeKind` represent linear graphs with optional conditional transitions backed by async predicates.
- **Execution Runtime**: `FlowExecutor` schedules agent nodes with bounded concurrency, handles tool invocations, and maintains message history in `FlowContext`.
- **Agents**: `Agent` trait produces `AgentAction` variants (`Next`, `Branch`, `CallTool`, `Finish`, `Continue`) with JSON string payloads. `AgentContext` exposes `FlowContext` plus `AgentRuntime`.
- **Tools**: `Tool` trait accepts `ToolInvocation` (`serde_json::Value` payloads) and returns `AgentMessage`. Registry stores simple `Arc<dyn Tool>` without capability metadata.
- **Context Store**: `FlowContext` wraps a `ContextStore` trait (memory and optional Redis implementation) with message history helpers.
- **Tests**: `tests/runtime_tests.rs`, `tests/pure_rust_volume_tests.rs` cover flow execution and volume scenarios; `tests/json_config_flow_tests.rs` demonstrates JSON-driven workflow configuration; `tests/food_vision_analysis_tests.rs` demonstrates multi-agent vision analysis workflows.

### Stage 0 Deliverables
- Consolidated TODO list seeded in automation (Cursor todo) to track phases 1–5.
- Regression entry point `scripts/regression.sh` to run the current test matrix.
- Documentation (this file) capturing scope and baseline architecture.

### Regression Strategy
- Execute `cargo fmt --check` and `cargo clippy --all-targets --all-features` to ensure lint hygiene.
- Run full test matrix: `cargo test --all-targets --all-features`.
- Monitor outputs from existing integration suites (`runtime_tests`, `json_config_flow_tests`, `food_vision_analysis_tests`, `pure_rust_volume_tests`) as baseline.

### Next Steps
- Phase 1: Extend Flow DSL with decision, join, and loop constructs while introducing structured context APIs.
- Maintain backward compatibility via additive APIs and feature flags where necessary.


