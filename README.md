# AgentFlow

一个基于 Rust 的智能体工作流框架，支持多模型、多智能体协作。

## 特性

- 🎯 **统一配置系统**: 使用单一的 JSON 文件定义所有服务、智能体和工作流
- 🔗 **图结构设计**: 基于 nodes 和 edges 的图结构，灵活定义工作流
- 🤖 **多模型支持**: 支持 Qwen、Moonshot、BigModel 等多种 LLM 模型
- 🔀 **多种节点类型**: 支持 Agent、Decision、Join、Loop、Terminal 等节点类型
- ⚡ **条件转换**: 支持基于状态的条件转换
- 👥 **多智能体协作**: 支持多个智能体顺序或并行协作
- 🤖 **自动路由**: 支持 LLM 驱动的智能路由决策

## 快速开始

### 安装

```bash
cargo build --features openai-client
```

### 配置

查看 `configs/graph_config_food_analysis.json` 或 `configs/graph_config_auto_routing.json` 了解配置格式。

参考 [项目文件结构说明](./docs/项目文件结构说明.md) 了解项目结构和配置格式。

### 运行示例

查看 `examples/` 目录了解使用示例。

### 运行测试

```bash
cargo test --features openai-client
```

## 文档

详细文档请查看 [docs/](./docs/) 目录：

- [项目文件结构说明](./docs/项目文件结构说明.md) - 项目结构和配置系统说明
- [路由和编排功能说明](./docs/路由和编排功能说明.md) - 路由和编排功能完整说明
- [自动和手动路由说明](./docs/自动和手动路由说明.md) - 路由模式对比
- [自动路由实现方案](./docs/自动路由实现方案.md) - 自动路由实现和使用指南
- [Documentation Index](./docs/README.md) - 文档索引

## 项目结构

```
agentflow/
├── src/
│   ├── config/              # 配置模块
│   │   ├── graph_config.rs  # 新的统一图配置
│   │   └── graph_loader.rs  # 配置加载器
│   ├── flow/                # 工作流执行引擎
│   ├── llm/                 # LLM 客户端
│   └── state/               # 状态管理
├── configs/
│   ├── graph_config_auto_routing.json     # 自动路由配置示例
│   └── graph_config_food_analysis.json    # 食物分析应用完整配置
└── docs/
    └── ...                       # 文档目录
```

## 使用示例

```rust
use agentflow::{GraphConfig, FlowContext, FlowExecutor, MessageRole, StructuredMessage};
use agentflow::state::MemoryStore;
use std::sync::Arc;
use std::fs;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config_json = fs::read_to_string("configs/graph_config_food_analysis.json")?;
    let graph_config = GraphConfig::from_json(&config_json)?;
    
    // 验证配置
    graph_config.validate()?;
    
    // 加载工作流
    let bundle = graph_config.load_workflow("workflow_food_analysis")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);

    // 创建初始消息
    let initial_message = StructuredMessage::new(json!({
        "user": "User",
        "goal": "Analyze this food image",
        "steps": []
    }))
    .into_agent_message(MessageRole::User, "client", Some("node_food_identifier".to_string()))?;

    // 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    println!("Workflow completed: {}", result.flow_name);
    Ok(())
}
```

## 核心功能

### ✅ 路由功能

- **决策节点**: 支持 `FirstMatch` 和 `AllMatches` 两种策略的条件路由
- **条件边**: 每条边都可以配置条件，只有满足条件时才执行
- **动态路由**: Agent 可以返回多个分支，支持动态路由选择

### ✅ 编排功能

- **并行编排**: Join 节点支持 `All`、`Any`、`Count(N)` 三种合并策略
- **循环编排**: Loop 节点支持条件循环和最大迭代次数限制
- **工具编排**: Tool Orchestrator 支持顺序、并行、故障转移三种策略
- **复杂编排**: 支持混合使用多种编排模式

### 支持的工作流类型

1. **链式流程**: 顺序执行的简单流程
2. **决策流程**: 包含决策节点的分支流程（✅ 路由功能）
3. **Join 流程**: 并行执行后合并的流程（✅ 编排功能）
4. **条件转换流程**: 基于条件的流程分支（✅ 路由功能）
5. **循环流程**: 包含循环的流程（✅ 编排功能）
6. **多智能体对话流程**: 多个智能体顺序对话
7. **多模型协作流程**: 使用不同模型的智能体协作

详细说明请参考 [路由和编排功能说明](./docs/路由和编排功能说明.md)

## 支持的模型

### Qwen (通义千问)
- **qwen-max** - 标准模型
- **qwen-vl-max** - 视觉模型

### Moonshot (月之暗面)
- **moonshot-v1-8k** - 标准模型
- **kimi-k2-turbo-preview** - 预览模型

### BigModel (智谱 AI)

**旗舰模型：**
- **glm-4.6** - 最新旗舰（355B 参数，200K 上下文）
- **glm-4.5** - 旗舰模型（128K 上下文）
- **glm-4.5-x** - 极速版本（100 tokens/s）
- **glm-4-plus** - 高智能旗舰

**高性价比模型：**
- **glm-4.5-air** - 轻量版本
- **glm-4.5-airx** - 极速轻量版本
- **glm-4.5-flash** - 免费版本 ⭐

**视觉推理模型：**
- **glm-4.5v** - 最强大的视觉推理模型
- **glm-4.1v-thinking-flash** - 10B 级最强视觉模型

**极速推理模型：**
- **glm-z1-airx** - 最快推理模型（200 tokens/s）
- **glm-z1-air** - 数学和逻辑推理优化
- **glm-z1-flash** - 完全免费

## 开发

### 运行测试

```bash
# 运行所有测试
cargo test --features openai-client

# 运行特定测试
cargo test --features openai-client test_chain_flow
```

### 代码检查

```bash
cargo check --features openai-client
cargo clippy --features openai-client
```

## 许可证

[添加许可证信息]

