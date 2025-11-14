## Phase 2：Agent Manifest 与类型化 I/O 指南（已实现）

> **状态**：✅ 已完成。所有功能已实现并通过测试。

本指南概述 AgentFlow Phase 2 引入的 Agent 能力描述、上下文访问与类型化消息机制。

### 1. Agent Manifest
- 结构定义：`src/agent/manifest.rs` 中的 `AgentManifest`、`AgentPort`、`AgentPortSchema`。
- 关键字段：
  - `name` / `description`：唯一标识与人类描述。
  - `inputs` / `outputs`：通过端口声明输入输出结构，可关联 Schema 名称。
  - `tools`：Agent 可直接调用的工具列表（名称需在 `ToolRegistry` 注册）。
  - `capabilities`：用于搜索或权限控制的标签。
- 使用方式：`AgentManifestBuilder` 提供链式 API，推荐在 Agent 注册时同步注册 Manifest。

### 2. Agent 输入输出类型
- `AgentInput<T>` 与 `AgentOutput<U>` 存放在 `src/agent/mod.rs`。
  - 提供 `try_from_message` / `into_message` 方法，自动处理 JSON 序列化和错误映射到 `AgentFlowError::Serialization`。
  - 适用于在 Agent 实现中直接获取结构化 Payload，避免手写 JSON 解析。
- `AgentMessage` 仍保留，可用于系统消息或向下兼容场景。

### 3. AgentContext 扩展
- API：
  - `session()`：跨节点共享的 `SessionContext`。
  - `variables()`：访问 `FlowVariables`，支持作用域内的状态读写。
  - `scope(kind)`：创建新的作用域，在 Loop/Branch 中自动清理。
- 生命周期钩子：
  - `on_start`、`on_message`、`on_finish`（默认空实现），可以在 FlowExecutor 中调用，详见 `tests/runtime_tests.rs`。

### 4. Manifest 与 Schema 结合
- Manifest `inputs`/`outputs` 中的 `schema` 字段可引用 `SchemaRegistry` 中注册的名称。
- 在 Flow/Tool 编排或 CLI 导出时，可联动 Schema 信息，提升静态验证能力。

### 5. 推荐实践
1. 为每个业务 Agent 配置 Manifest，并在初始化阶段统一注册，便于插件化与 CLI 展示。
2. 使用 `AgentInput<T>` 与 `AgentOutput<T>` 定义稳定的消息契约；搭配 `StructuredMessage<T>` 在 Flow/Tool 之间传递复杂数据。
3. 在 `on_start`/`on_finish` 钩子中管理资源或写入日志，确保 Agent 生命周期透明可追踪。

### 6. 后续计划
- 与 Phase 5 CLI 集成：`agentflow plugins list` 未来将展示 Agent Manifest 概要。
- Manifest 校验：结合 `SchemaRegistry::validate`，在 Flow 构建阶段提前发现类型问题。

更多示例请参考：
- `tests/agent_manifest_tests.rs`
- `tests/agent_typed_io_tests.rs`
- `tests/runtime_tests.rs::agent_lifecycle_hooks_are_invoked_once`

如需补充更详细的示例或模板，请在 Phase 5 文档任务中记录需求。 

