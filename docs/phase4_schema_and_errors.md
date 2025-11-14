## 阶段 4：统一 Schema、消息封装与错误处理（已实现）

> **状态**：✅ 已完成。所有功能已实现并通过测试。

### 目标概述
- 引入统一的 Schema 表达（JSON Schema / 自研结构），贯通 Flow/Agent/Tool 的输入输出声明与校验。
- 标准化消息封装工具，减少手写 JSON 字符串，提供类型化/结构化输出。
- 建立一致的错误格式：包含错误码、严重级别、上下文信息，可用于重试、熔断和日志追踪。

---

### 1. Schema 管理

#### 1.1 Schema 抽象
- 新增 `Schema` 枚举或结构，描述基本类型（string/number/object/...）以及嵌套结构。
- `SchemaRegistry`：集中存储 schema 定义，可从 Manifest 或 Flow 描述中注册。
- 支持 JSON Schema 导入：将 Manifest/Flow 中的 `json_schema` 字段转换为内部表征；缺省时生成基础 schema。

#### 1.2 校验流程
- Flow 构建时校验：`FlowBuilder::validate_parameters()` 比对节点输入输出与 schema；`ToolNode` 校验 `ToolManifest`。
- Agent 发送/接收消息时：`AgentInput<T>` / `AgentOutput<T>` 可以在序列化/反序列化后执行 schema 校验（可配置）。
- Orchestrator 调用工具前后：`ToolOrchestrator::validate_input/output` 引入 schema 解析与验证结果（错误码如 `schema.validation_failed`）。

#### 1.3 Schema 生成 & 文档
- 提供宏或 derive：`#[derive(Schema)]` 自动生成 schema 信息。
- `schema::export` 工具输出所有 Schema/Manifest 组合，供文档/前端使用。

---

### 2. 统一消息封装

#### 2.1 TypedMessage / StructuredMessage
- `StructuredMessage<T>`：包含 `payload: T`, `schema: SchemaRef`, `metadata: Value`。
- Agent/Tool 返回 `StructuredMessage<T>`，框架负责渲染为 `AgentMessage`，保留原始结构便于下游解析。

#### 2.2 消息通道约束
- FlowContext 存储结构化消息历史，提供 `history_typed<T>()`。
- Tool/Agent 调用 Pipeline 时自动透传 schema 信息，支持节点/工具之间结构化对接。

---

### 3. 错误处理与诊断

#### 3.1 统一错误结构
- 新增 `FrameworkError`：
  ```rust
  struct FrameworkError {
      code: String,             // e.g. schema.validation_failed
      message: String,
      severity: ErrorSeverity,  // Info/Warning/Error
      context: Value,           // additional data
  }
  ```
- 与 `AgentFlowError` 整合：保留枚举，对外输出 `FrameworkError` 对象；提供 `to_framework_error()`。

#### 3.2 错误来源分类
- Schema 校验错误：`code = "schema.validation_failed"`。
- 资源限制：`code = "resource.limited"`。
- Tool 调用失败：`code = "tool.execution_failed"` + manifest 信息。

#### 3.3 日志与追踪
- 为 FlowExecutor、ToolOrchestrator 添加 `tracing` 埋点，记录错误结构。
- Flow 执行结果 `FlowExecution` 增加 `errors: Vec<FrameworkError>`，用于 UI/调试。

---

### 4. 实施路线
1. 引入 Schema 模块与统一错误结构（无强校验，仅结构）。
2. Agent/Tool/Flow builder 接入 schema 引用与基本校验。
3. Tool Orchestrator / FlowExecutor 接入错误映射与输出。
4. 扩展测试：schema 校验失败、错误码传递、结构化消息处理。

---

### 5. 测试计划
- 单元测试：
  - Schema 注册/转换（JSON Schema -> 内部结构）。
  - AgentInput/Output 在 schema 不匹配时返回 `FrameworkError`。
  - ToolOrchestrator 对输入输出校验并生成错误码。
- 集成测试：
  - Flow 中某节点返回结构化消息，下游工具消费并校验。
  - Schema 错误导致 Flow 提前终止并在结果中包含错误信息。

---

### 6. 文档与工具
- 更新 `docs/phase4_schema_and_errors.md`（本文件）随时记录实现进度。
- 在 README/文档中增加“Schema/错误处理”章节，并提供 sample output。
- CLI/脚本：`cargo run -- schema export` 输出所有 schema & manifest 数据，便于前端或监控引用。

