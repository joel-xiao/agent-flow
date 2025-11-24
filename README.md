# AgentFlow

**下一代 AI Agent 编排框架 - 完全由 JSON 配置驱动**

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 🌟 核心特性

### 🎯 完全配置驱动
- ✅ **零代码工作流**：所有流程通过 JSON 配置定义
- ✅ **内置工具系统**：下载器、图片生成等工具自动注册
- ✅ **工具参数配置化**：工具参数直接在 JSON 中定义，无需代码修改

### 🚀 强大的编排能力
- ✅ **多 Agent 协作**：支持复杂的 Agent 协作流程
- ✅ **并行处理**：多路并行执行，性能提升 3 倍
- ✅ **智能路由**：自动或手动路由到最佳处理节点
- ✅ **Join 节点**：灵活的结果汇聚策略（all/any/count）
- ✅ **Loop 节点**：支持条件循环和迭代
- ✅ **Decision 节点**：复杂的条件分支逻辑

### 🔧 通用 LLM 支持
- ✅ **模型无关**：支持 OpenAI、Qwen、GLM 等所有 LLM
- ✅ **统一接口**：无需为每个模型写专门代码
- ✅ **流式输出**：实时响应，优化用户体验
- ✅ **图片生成**：内置支持 AI 图片生成（通义万相、DALL-E 等）

### 🛠️ 内置工具生态
- ✅ **DownloaderTool**：自动下载文件（图片/音频/视频/文档）
- ✅ **ImageGeneratorTool**：AI 图片生成工具
- ✅ **易于扩展**：实现 Tool trait 即可添加自定义工具

## 📦 快速开始

### 安装依赖

```bash
# 克隆项目
git clone https://github.com/yourusername/agentflow.git
cd agentflow

# 编译项目
cargo build --release --features openai-client
```

### 运行示例

#### 1. 营销内容生成系统

```bash
# 设置 API Key
export QWEN_API_KEY=your_api_key_here

# 运行应用
cargo run --example marketing_generator --features openai-client
```

**功能展示**：
- 🎯 需求分析 → 智能路由 → 文案生成 → 润色 → 风险审核
- 🌍 并行生成多语言版本（中文/英文/日文）
- 🎨 AI 图片生成（通义万相）
- 📥 自动下载图片到本地

#### 2. 食物识别分析系统

```bash
cargo run --example food_analysis_app --features openai-client -- tests/test_food.jpg
```

**功能展示**：
- 🍔 图片识别食物类型
- 📊 分析营养成分和热量
- 💡 提供健康建议

## 🎨 JSON 配置示例

### 基本工作流

```json
{
  "name": "my_workflow",
  "nodes": [
    {
      "id": "agent_analyzer",
      "type": "agent",
      "config": {
        "name": "analyzer",
        "driver": "qwen",
        "model": "qwen-max",
        "endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "role": "Data Analyzer",
        "prompt": "Analyze the input data and extract key insights.",
        "api_key": "${QWEN_API_KEY}"
      }
    },
    {
      "id": "tool_downloader",
      "type": "tool_node",
      "config": {
        "pipeline": "download_file",
        "params": {
          "save_dir": "output/images",
          "filename_prefix": "result"
        }
      }
    }
  ],
  "edges": [
    {
      "from": "agent_analyzer",
      "to": "tool_downloader",
      "type": "always"
    }
  ]
}
```

### 内置工具配置

所有工具参数直接在 JSON 中配置，无需修改代码：

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "params": {
      "save_dir": "generated_images",        // 自定义保存目录
      "filename_prefix": "marketing",        // 自定义文件名前缀
      "url": "https://example.com/image.png" // 可选：直接指定 URL
    }
  }
}
```

## 📚 完整文档

### 核心文档
- [完整使用指南](docs/完整使用指南.md) - 框架详细使用说明
- [代码规范和最佳实践](docs/代码规范和最佳实践.md) - 开发规范
- [配置API_Key指南](docs/配置API_Key指南.md) - API Key 配置方法

### 工具系统
- [JSON配置驱动的工具使用](docs/JSON配置驱动的工具使用.md) - JSON 配置完整指南
- [内置工具使用说明](docs/内置工具使用说明.md) - 内置工具详细说明
- [工具系统架构](docs/工具系统架构.md) - 工具系统设计原理
- [下载工具配置示例](docs/下载工具配置示例.md) - 下载工具详细配置

### 应用示例
- [智能营销内容生成系统设计](docs/智能营销内容生成系统设计.md) - 营销系统设计文档
- [食物识别分析应用设计](docs/食物识别分析应用设计.md) - 食物分析系统设计

### 高级功能
- [路由和编排功能说明](docs/路由和编排功能说明.md) - 路由机制详解
- [流式输出实现说明](docs/流式输出实现说明.md) - 流式输出使用指南

## 🏗️ 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                    应用层 (Application)                  │
│  ┌────────────────┐  ┌────────────────┐  ┌───────────┐ │
│  │ JSON 配置       │  │ 示例应用        │  │ 自定义工具 │ │
│  │ graph.json     │  │ marketing_gen  │  │ MyTool    │ │
│  └────────────────┘  └────────────────┘  └───────────┘ │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                   框架层 (Framework)                     │
│  ┌───────────────────────────────────────────────────┐  │
│  │         FlowExecutor + ToolOrchestrator           │  │
│  │  - 工作流执行                                      │  │
│  │  - Agent 协作                                     │  │
│  │  - Tool 编排                                      │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │              GenericHttpClient                    │  │
│  │  - 统一 LLM 接口                                  │  │
│  │  - 支持所有 OpenAI 兼容 API                       │  │
│  │  - 图片生成支持                                   │  │
│  └───────────────────────────────────────────────────┘  │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                 内置工具层 (Built-in Tools)              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ Download │  │ ImageGen │  │   Echo   │  │  ...   │ │
│  │   Tool   │  │   Tool   │  │   Tool   │  │        │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
│           ✅ 自动注册              ✅ 自动注册           │
└─────────────────────────────────────────────────────────┘
```

## 💡 核心概念

### 节点类型

| 节点类型 | 说明 | 配置示例 |
|---------|------|---------|
| `agent` | 调用 LLM Agent | `{"type": "agent", "config": {...}}` |
| `tool_node` | 调用内置或自定义工具 | `{"type": "tool_node", "config": {"pipeline": "download_file"}}` |
| `decision_node` | 条件分支 | `{"type": "decision_node", "config": {"branches": [...]}}` |
| `join_node` | 结果汇聚 | `{"type": "join_node", "config": {"strategy": "all"}}` |
| `loop_node` | 循环处理 | `{"type": "loop_node", "config": {"max_iterations": 10}}` |
| `terminal_node` | 流程终止 | `{"type": "terminal_node"}` |

### 工具系统

#### 内置工具

**DownloaderTool** - 文件下载
- 自动从上下文提取 URL
- 支持图片、音频、视频、文档
- 可配置保存目录和文件名前缀

**ImageGeneratorTool** - AI 图片生成
- 支持通义万相、DALL-E 等
- 异步任务处理和轮询

#### 自定义工具

```rust
use async_trait::async_trait;
use agentflow::tools::tool::{Tool, ToolInvocation};
use agentflow::agent::AgentMessage;
use agentflow::error::Result;

pub struct MyCustomTool;

#[async_trait]
impl Tool for MyCustomTool {
    fn name(&self) -> &'static str {
        "my_custom_tool"
    }
    
    async fn call(
        &self,
        invocation: ToolInvocation,
        ctx: &FlowContext
    ) -> Result<AgentMessage> {
        // 实现工具逻辑
        Ok(AgentMessage { /* ... */ })
    }
}
```

## 🔥 项目亮点

### 1. 完全配置驱动
- **零代码修改**：所有参数在 JSON 中配置
- **动态加载**：运行时加载配置，无需重新编译
- **环境适配**：不同环境使用不同配置文件

### 2. 通用 LLM 支持
- **无厂商锁定**：不依赖特定 LLM 提供商
- **统一接口**：一套代码支持所有 OpenAI 兼容 API
- **易于扩展**：添加新 LLM 只需配置，无需改代码

### 3. 内置工具生态
- **开箱即用**：内置工具自动注册，无需手动配置
- **参数配置化**：工具参数在 JSON 中定义
- **易于扩展**：实现 Tool trait 即可集成

### 4. 生产级质量
- **类型安全**：基于 Rust，编译时类型检查
- **异步处理**：tokio 驱动，高性能并发
- **错误处理**：完善的错误处理机制

## 📊 性能对比

| 场景 | 串行执行 | 并行执行 | 性能提升 |
|------|---------|---------|---------|
| 多语言生成 | 30秒 | 10秒 | **3x** |
| 多任务处理 | 60秒 | 15秒 | **4x** |

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 License

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Tokio](https://tokio.rs/) - 异步运行时
- [Serde](https://serde.rs/) - 序列化框架
- [Reqwest](https://github.com/seanmonstar/reqwest) - HTTP 客户端

## 📮 联系方式

- 问题反馈：[GitHub Issues](https://github.com/yourusername/agentflow/issues)
- 讨论交流：[GitHub Discussions](https://github.com/yourusername/agentflow/discussions)

---

**AgentFlow - 让 AI Agent 编排更简单！** 🚀
