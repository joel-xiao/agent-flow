## 阶段 1：Flow DSL 与执行器扩展（已实现）

> **状态**：✅ 已完成。所有功能已实现并通过测试。

### 目标概述
- 允许 Flow 图描述条件分支、多入聚合以及循环结构，支持并行执行。
- 引入结构化上下文 API，支持 session 级、节点级、分支级变量管理。
- 保持现有线性 Flow 构建方式兼容。

---

### 1. 新增节点类型定义

#### 1.1 `FlowNodeKind` 扩展
- 添加枚举变体：
  - `Decision { policy: DecisionPolicy, branches: Vec<DecisionBranch> }`
  - `Join { strategy: JoinStrategy, inbound: Vec<String> }`
  - `Loop { spec: LoopSpec }`
- `DecisionBranch`：`name`, `condition`, `target`, 可选 `metadata`。
- `JoinStrategy`：`All`, `Any`, `Count(usize)`, `Custom(Arc<dyn JoinHandler>)`。
- `LoopSpec`：`entry`, `condition`, `max_iterations`, `break_targets`。

#### 1.2 节点元数据
- `FlowNode` 增加可选 `metadata: Value`（feature gated? 默认 `None`），便于 DSL 注入额外参数。
- `FlowNode` 保留 `name` 与 `kind`，以避免现有 API 破坏。

---

### 2. FlowBuilder API 设计

#### 2.1 新增方法
- `add_decision_node(name, policy) -> DecisionBuilder`
  - 支持链式 `.branch("positive", |b| b.condition(cond).to("agent_a"))`
- `add_join_node(name, strategy)`
  - `.require_from(&["node_a", "node_b"])`
- `add_loop_node(name, LoopSpecBuilder)`
  - `LoopSpecBuilder::new("start").condition(loop_condition).max_iterations(32)`
- `connect_if(from, to, condition)`：语法糖，内部构造 `Decision`。
- `connect_join(source, join_node)`：注册 join inbound。
- `connect_loop_entry(loop_node, entry_target)` 与 `connect_loop_exit(loop_node, exit_target)`.

#### 2.2 构建器体验
- `FlowBuilder` 保持原有 `add_agent_node` / `connect` 方法。
- 新 builder 对象在 `build()` 调用时回写至内部 `nodes`/`transitions`。
- 需要额外校验：
  - `Join` inbound 节点是否存在。
  - `Loop` entry / break targets合法。
  - `Decision` 至少包含一个分支。

---

### 3. FlowExecutor 调度扩展

#### 3.1 调度状态表示
- 引入新的内部枚举 `SchedulerNodeKind` 与 `ExecutionState`：
  - `ExecutionState::Decision { pending_branches, policy }`
  - `ExecutionState::Join { collected, strategy }`
  - `ExecutionState::Loop { spec, iteration, in_progress }`
- `FlowEvent` 增加 `trace_id` / `branch_id`，用于 join 聚合关联。

#### 3.2 并行 & 聚合
- `Decision` 触发多分支事件后，调度器根据 `DecisionPolicy` 控制：
  - `FirstReady`：首个完成即进入下一节点，其余取消/标记完成。
  - `AllBranches`：等待全部分支完成，收敛到 join 或父节点。
- `Join` 节点维护 `JoinTracker`（`HashMap<trace, JoinState>`），记录已完成分支。
- 完成后根据策略生成聚合消息：
  - 默认聚合：合并所有消息内容到 JSON 数组，或触发自定义 handler。

#### 3.3 循环管理
- `LoopSpec` 提供 `condition: LoopConditionFn(FlowContext) -> bool`。
- 进入循环时初始化 `LoopFrame`（`iteration`, `accumulator` 等）。
- 每次迭代后评估条件；超出 `max_iterations` 返回错误 `AgentFlowError::LoopBoundExceeded`.
- 支持 `break`/`continue`：`AgentAction` 新增 `LoopControl` 变体？或保留 `Next` 指向 break target。

#### 3.4 兼容性
- 现有线性流程无需 `Decision/Join/Loop`，调度逻辑通过 `match` 落到默认分支。
- 旧 `Transition` 条件仍可用；新节点采用独立处理路径，避免改变 `next_from_flow` 的默认行为。

---

### 4. 上下文与参数管理

#### 4.1 `FlowContext` 扩展
- 添加 `FlowContext::session()` 返回 `SessionContext`（拥有全局 KV）。
- 支持作用域栈：
  - `ScopeHandle` 表示节点/分支作用域，生命周期自动 drop 时清理。
  - `FlowExecutor` 在进入节点时 push scope，离开时 pop。
- 新增 `FlowContext::variables()` 暴露 `FlowVariableStore`（支持 `set/get/remove` 带级别枚举）。

#### 4.2 `FlowParameter` / `FlowVariable`
- `FlowParameter` 描述 Flow 输入输出：`name`, `kind`（`Input` / `Output` / `InOut`）, `type_info`.
- `FlowBuilder::with_parameter(param)` 注册输入输出声明；`Flow` 存储 metadata。
- `FlowVariable` 表示运行期变量：`identifier`, `scope`, `default`.

#### 4.3 API 示例
```rust
builder
    .with_parameter(FlowParameter::input::<OrderRequest>("order"))
    .with_parameter(FlowParameter::output::<OrderConfirmation>("confirmation"));

ctx.session().set("user_id", "123").await?;
let node_scope = ctx.scope(FlowScope::Node(node_name));
node_scope.set("attempt", 1).await?;
```

---

### 5. 错误处理与监控挂钩
- 新增错误类型：
  - `AgentFlowError::DecisionPolicyViolation`
  - `AgentFlowError::JoinTimeout`
  - `AgentFlowError::LoopBoundExceeded`
- 在 `FlowExecutor` 中引入 tracing 埋点：`decision.start`, `decision.end`, `join.waiting`, `loop.iteration`.
- 考虑为 `FlowExecution` 增加 `trace` 信息（`HashMap` 或 `Vec<FlowTraceEvent>`）用于调试。

---

### 6. 测试计划
- 单元测试：
  - `decision_node_executes_matching_branch`
  - `join_node_waits_for_all_required_branches`
  - `loop_node_stops_after_max_iterations`
- 集成测试：
  - 条件 + 循环组合流。
  - 并行分支 + 聚合。
  - 新上下文 API 读写验证（节点作用域隔离）。
- 回归测试：`scripts/regression.sh`；确保旧示例不修改即可通过。

---

### 7. 实施步骤（建议顺序）
1. ✅ 扩展 Flow 数据结构（节点类型、构建器、元数据）。
2. ✅ 实现执行器调度的 Decision/Join 支持，补充基础测试。
3. ✅ 引入 Loop 处理与上下文作用域扩展。
4. ✅ 增加 Flow 参数声明与上下文 API，实现必要示例。
5. ✅ 全面测试与文档更新。

### 8. JSON 配置支持（新增）

除了程序化的 `FlowBuilder` API，AgentFlow 还支持通过 JSON 配置定义完整的工作流：

- **完整 JSON 配置**：通过 `load_workflow_from_value` 可以从 JSON 加载包含 Agent、Tool 和 Flow 的完整工作流。
- **节点类型支持**：所有节点类型（Agent、Decision、Join、Loop、Tool、Terminal）都支持 JSON 配置。
- **条件表达式**：支持多种条件类型（always、state_equals、state_not_equals 等）的 JSON 配置。
- **流式输出**：使用 LLM 驱动的 Agent 支持实时流式输出，提升用户体验。

详细使用说明请参考 `docs/json_workflow_configuration.md`。


