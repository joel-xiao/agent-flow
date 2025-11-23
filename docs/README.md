# AgentFlow Documentation

## 概述

AgentFlow 是一个基于 Rust 的智能体工作流框架，支持多模型、多智能体协作，通过统一的 JSON 配置来定义复杂的工作流。

## 核心特性

- **统一配置系统**: 使用单一的 JSON 文件定义所有服务、智能体和工作流
- **图结构设计**: 基于 nodes 和 edges 的图结构，灵活定义工作流
- **多模型支持**: 支持 Qwen、Moonshot、BigModel 等多种 LLM 模型
- **多种节点类型**: 支持 Agent、Decision、Join、Loop、Terminal 等节点类型
- **条件转换**: 支持基于状态的条件转换
- **多智能体协作**: 支持多个智能体顺序或并行协作
- **自动路由**: 支持 LLM 驱动的自动路由功能

## 快速开始

### 1. 安装

```bash
cargo build --features openai-client
```

### 2. 配置

编辑 `configs/graph_config_example.json`，定义你的服务、智能体和工作流。

### 3. 运行测试

```bash
cargo test --features openai-client
```

## 文档索引

### 核心文档

- [代码评估报告](./代码评估报告.md) - 代码结构评估和是否需要进一步改造（✅ 已完成）
- [目录结构检查报告](./目录结构检查报告.md) - 目录结构合理性检查报告
- [项目文件结构说明](./项目文件结构说明.md) - 项目文件结构详细说明

### 功能文档

- [路由和编排功能说明](./路由和编排功能说明.md) - 路由和编排功能的完整说明
- [自动和手动路由说明](./自动和手动路由说明.md) - 两种路由模式的对比和说明
- [自动路由实现方案](./自动路由实现方案.md) - 自动路由的详细实现方案

### 应用示例

- [食物识别分析应用设计](./食物识别分析应用设计.md) - 完整应用设计文档（使用所有功能）


## 项目结构

```
agentflow/
├── src/
│   ├── config/              # 配置模块 ✅
│   │   ├── graph_config.rs  # 统一图配置（重新导出）
│   │   ├── graph.rs         # 核心图结构
│   │   ├── conditions.rs    # 条件定义
│   │   ├── nodes.rs         # 节点配置
│   │   ├── agent_config.rs  # Agent 配置
│   │   ├── agent_rules.rs   # Agent 规则
│   │   └── graph_loader.rs  # 配置加载器
│   ├── flow/                # 工作流执行引擎 ✅
│   │   ├── config/         # 配置结构
│   │   ├── agent/          # Agent实现
│   │   ├── loader/         # 工作流加载
│   │   └── services/       # 服务模块
│   ├── llm/                # LLM客户端 ✅
│   │   ├── http/           # HTTP 客户端实现
│   │   │   ├── generic.rs  # GenericHttpClient
│   │   │   ├── stream.rs   # SSE 流式响应
│   │   │   └── configs.rs  # 端点配置
│   │   └── extended/       # 扩展功能
│   ├── runtime/            # 运行时执行引擎 ✅
│   │   ├── executor.rs     # 执行器
│   │   ├── handlers.rs     # 节点处理
│   │   └── ...            # 其他模块
│   └── state/              # 状态管理
├── configs/
│   ├── graph_config_example.json  # 配置示例
│   └── graph_config_with_routing.json  # 路由配置示例
└── docs/
    └── ...                 # 文档目录
```

## 支持的工作流类型

1. **链式流程**: 顺序执行的简单流程
2. **决策流程**: 包含决策节点的分支流程
3. **Join 流程**: 并行执行后合并的流程
4. **条件转换流程**: 基于条件的流程分支
5. **循环流程**: 包含循环的流程
6. **多智能体对话流程**: 多个智能体顺序对话
7. **多模型协作流程**: 使用不同模型的智能体协作

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

## 重构成果

所有大文件重构已完成：
- ✅ `flow/loader.rs`: 913行 → 15行
- ✅ `flow/mod.rs`: 598行 → 27行
- ✅ `llm/extended/client.rs`: 1196行 → 223行
- ✅ `runtime/mod.rs`: 793行 → 12行
- ✅ `config/graph_config.rs`: 512行 → 59行
- ✅ `flow/services/routing.rs`: 424行 → 48行
- ✅ `state/mod.rs`: 381行 → 14行
- ✅ `schema/mod.rs`: 267行 → 42行

详细内容请参考 [代码评估报告](./代码评估报告.md) 和 [目录结构检查报告](./目录结构检查报告.md)

## 许可证

[添加许可证信息]
