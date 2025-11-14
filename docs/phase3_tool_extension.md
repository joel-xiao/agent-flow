## 阶段 3：Tool Manifest、编排与资源管理（已实现）

> **状态**：✅ 已完成。所有功能已实现并通过测试。

### 目标概述
- 为 Tool 体系引入 Manifest 描述，可声明输入输出 schema、权限/资源要求。
- 构建 Tool Orchestrator，支持并行调用、超时/重试/fallback 策略以及流水线定义。
- 搭建统一的 ModelRegistry / ToolResourceManager，管理模型实例、API client 与并发配额。
- 与 Flow/Agent 深度集成，实现 ToolFlow 节点和简化调用体验。

---

### 1. Tool Manifest 设计

#### 1.1 数据结构
- `ToolManifest`：
  - `name`（必填）、`description`。
  - `inputs`/`outputs`：`ToolPort`，包含 schema/描述/示例。
  - `capabilities`：`vec!["llm.completion", "search"]` 等能力标签。
  - `permissions`：所需外部权限（API scope、文件系统）。
  - `resources`：引用资源池标识（模型、连接池、速率限制器）。
- `ToolPortSchema`：`type_name`、`format`（如 `json`, `text`, `binary`）、`json_schema`。
- `ToolManifestBuilder`：链式 API，保持与 Agent Manifest 一致的风格。

#### 1.2 Manifest 校验
- 在 Tool 注册时校验 Manifest（必填字段、重复端口名）。
- Flow 构建时，如果节点声明的 tool 输入输出与 manifest 不匹配，提供编译期/构建期提示。
- 可选：`--check-manifests` CLI 扫描 manifest 完整性。

---

### 2. Tool Orchestrator

#### 2.1 核心能力
- `ToolOrchestrator` trait/结构：
  - `register_pipeline(name, ToolPipeline)`.
  - `execute(name, ToolInvocationContext)` 返回 `AgentMessage` 或结构化结果。
- 支持策略：
  - 并行调用（`Parallel(vec![task])`），聚合策略（first success/all success）。
  - 超时控制：每个步骤可定义超时，超时触发 fallback。
  - 重试策略：指数退避、固定次数。
  - Fallback：顺序尝试多个工具。
- `ToolPipeline` DSL：描述节点、条件、依赖，内部使用 orchestrator 调度。

#### 2.2 调度实现
- 使用 `tokio::time`、`JoinSet`/`FuturesOrdered` 实现并行。
- 失败记录：返回结构化错误（错误码、步骤名称、重试次数）。
- 与 Manifest 联动：验证 pipeline 中引用的工具均已注册且 manifest 可用。

---

### 3. 资源管理

#### 3.1 ModelRegistry
- 抽象模型实例（LLM、Embedding、语音等）。
- 支持：
  - 惰性加载：首次使用时初始化 client。
  - 并发控制（Semaphore）、速率限制（tokio::sync::Semaphore 或自定义 limiter）。
  - 元数据：API Key、base_url，通过配置或环境变量注入。
  - **安全提示**：API Key 应优先通过环境变量 `QWEN_API_KEY` 提供，避免硬编码在配置文件中。
- API 示例：
  ```rust
  registry.register("openai.gpt4", ModelSpec::OpenAI { key: secret, model: "gpt-4" });
  let client = registry.checkout("openai.gpt4").await?;
  ```

#### 3.2 ToolResourceManager
- 管理非模型资源：数据库连接、HTTP client、文件缓存。
- 与 Tool Manifest 链接：Manifest 中 `resources` 列表要求 orchestrator 在执行前申请资源。
- 支持自动释放/回收，确保工具调用结束后资源归还池中。

---

### 4. Flow / Agent 集成

#### 4.1 ToolFlow 节点
- 在 Flow DSL 中新增 `Tool` 节点类型，允许描述工具流水线：
  ```rust
  builder
      .add_tool_node("summarizer", "summarize.pipeline")
      .connect("ingest", "summarizer");
  ```
- Flow 执行器在遇到 `Tool` 节点时调用 orchestrator，并将结果封装为 `AgentMessage` 或类型化输出。

#### 4.2 Agent 简化调用
- `AgentRuntime::call_pipeline(name, context)` 直接触发 orchestrator，减少手写 ToolInvocation。
- `AgentContext` 提供 `session().set`/`variables()` 帮助 orchestrator 记录状态。

---

### 5. 测试计划
- 单元测试：
  - Manifest builder & 校验（缺省字段 / 重复端口）。
  - Orchestrator 超时/重试/fallback 逻辑。
  - ModelRegistry 并发限流。
- 集成测试：
  - Flow 节点调用 ToolFlow 并行工作流程。
  - Agent 使用 pipeline + session 变量。
- 文档/示例：
  - 更新 README，新增 `examples/tool_pipeline.rs`。
  - 记录常见错误诊断（超时、资源不足）。

---

### 6. 实施顺序建议
1. 实现 Tool Manifest 数据结构与校验（当前工作项）。
2. 开发 Tool Orchestrator 核心调度逻辑及测试。
3. 构建 ModelRegistry / ToolResourceManager 并接入 orchestrator。
4. 整合 Flow/Agent API，提供示例与文档更新。
5. 执行全量回归 `scripts/regression.sh`，确保兼容性。

