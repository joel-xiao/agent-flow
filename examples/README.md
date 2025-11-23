# AgentFlow 使用示例

本目录包含基于 GraphConfig（nodes 和 edges）的工作流示例。

## 示例列表

### 1. routing_workflow.rs
**路由工作流示例**

演示如何使用 Decision 节点实现条件路由。

**工作流结构**：
- Agent → Decision → (Route A | Route B) → Join → Terminal

**运行方式**：
```bash
cargo run --example routing_workflow --features openai-client
```

**注意**：此示例使用内联配置，不依赖外部 JSON 文件。

### 2. auto_routing_workflow.rs
**自动路由工作流示例**

演示如何使用自动路由功能（LLM 驱动的路由）。

**工作流结构**：
- Router Agent → (自动路由到多个目标) → Join → Terminal

**配置文件**：`configs/graph_config_auto_routing.json`

**运行方式**：
```bash
cargo run --example auto_routing_workflow --features openai-client
```

**注意**：此示例依赖外部配置文件。

### 3. decision_workflow.rs
**决策节点工作流示例**

演示如何使用 Decision 节点实现多分支路由。

**工作流结构**：
- Agent → Decision (FirstMatch/AllMatches) → Multiple Branches → Join

**运行方式**：
```bash
cargo run --example decision_workflow --features openai-client
```

**注意**：此示例使用内联配置，不依赖外部 JSON 文件。

### 4. join_workflow.rs
**Join 节点工作流示例**

演示如何使用 Join 节点实现并行处理和结果合并。

**工作流结构**：
- Splitter → (Worker A, Worker B) → Join → Terminal

**运行方式**：
```bash
cargo run --example join_workflow --features openai-client
```

**注意**：此示例使用内联配置，不依赖外部 JSON 文件。

### 5. loop_workflow.rs
**Loop 节点工作流示例**

演示如何使用 Loop 节点实现循环处理。

**工作流结构**：
- Entry → Loop (Condition) → Exit

**运行方式**：
```bash
cargo run --example loop_workflow --features openai-client
```

**注意**：此示例使用内联配置，不依赖外部 JSON 文件。

### 6. food_analysis_app.rs
**完整应用示例**

完整的食物识别分析应用，使用所有 AgentFlow 功能：
- 自动路由
- Decision 节点
- Join 节点
- 并行处理
- 流式输出

**配置文件**：`configs/graph_config_food_analysis.json`

**运行方式**：
```bash
cargo run --example food_analysis_app --features openai-client
```

详细说明请参考：[食物识别分析应用设计](../docs/食物识别分析应用设计.md)

## 配置说明

所有示例都基于 `GraphConfig` 格式，使用 `nodes` 和 `edges` 定义工作流：

```json
{
  "name": "workflow_name",
  "nodes": [
    {
      "id": "node_id",
      "type": "node_type",
      "config": { ... },
      "workflow": "workflow_id"
    }
  ],
  "edges": [
    {
      "from": "node_a",
      "to": "node_b",
      "type": "always",
      "workflow": "workflow_id"
    }
  ]
}
```

## 节点类型

- `service` - 服务配置（LLM 服务等）
- `agent` - Agent 定义
- `agent_node` - 工作流中的 Agent 节点
- `decision_node` - 决策节点
- `join_node` - 合并节点
- `loop_node` - 循环节点
- `terminal_node` - 终止节点
- `workflow` - 工作流定义

## 边类型

- `always` - 总是执行
- `conditional` - 条件执行（需要 condition 配置）

## 环境变量

部分示例需要设置环境变量：

```bash
export QWEN_API_KEY="your-api-key"
export QWEN_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
export QWEN_MODEL="qwen-max"
```

## 更多信息

详细配置说明请参考：
- [项目文件结构说明](../docs/项目文件结构说明.md)
- [路由和编排功能说明](../docs/路由和编排功能说明.md)
- [自动路由实现方案](../docs/自动路由实现方案.md)

