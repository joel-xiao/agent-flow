# AgentFlow 示例程序

本目录包含 AgentFlow 框架的核心应用示例。

## 📚 应用列表

### 🎨 智能营销内容生成系统

**文件**: `marketing_generator.rs`  
**配置**: `../configs/graph_config_marketing_generator.json`  
**功能**: 完整的营销内容生成流水线，支持可选的图片生成

```bash
# 基础模式：生成多语言文案和图片提示词
cargo run --example marketing_generator --features openai-client

# 完整模式：生成文案、提示词和真实图片
cargo run --example marketing_generator --features openai-client -- --generate-image
```

**核心功能**:
- ✅ 需求分析 - 提取产品特征和目标受众
- ✅ 智能路由 - 自动选择最佳文案风格
- ✅ 文案生成 - 创作营销文案
- ✅ 质量把控 - 风险审核和内容润色
- ✅ 并行翻译 - 同时生成中英日三语版本
- ✅ Join汇总 - 多语言结果合并
- ✅ 图片提示词生成 - AI图片描述
- ✅ 图片生成 - 通义万相图片生成（可选）
- ✅ 结果汇总 - 完整输出报告

**详细文档**: [README_MARKETING_GENERATOR.md](./README_MARKETING_GENERATOR.md)

---

### 🍕 食物识别分析应用

**文件**: `food_analysis_app.rs`  
**配置**: `../configs/graph_config_food_analysis.json`  
**功能**: 基于视觉模型的食物识别和营养分析

```bash
cargo run --example food_analysis_app --features openai-client
```

**核心功能**:
- ✅ 视觉模型集成 - 使用 qwen-vl-max 进行图片分析
- ✅ 图片预处理 - 质量检测
- ✅ 食物识别 - 识别图片中的所有食物
- ✅ 循环重试 - Loop节点实现置信度检查和自动重试
- ✅ 智能路由 - 根据需求选择简单或详细分析
- ✅ 并行分析 - 同时进行分量分析和营养分析
- ✅ Join汇总 - 合并分析结果
- ✅ 卡路里计算 - 详细的热量统计
- ✅ 结果汇总 - 生成完整的分析报告和健康建议

---

## 🚀 快速开始

### 1. 安装依赖

```bash
# 确保已安装 Rust（1.70+）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆仓库
git clone <repo_url>
cd agentflow
```

### 2. 配置 API Key

```bash
# 设置通义千问 API Key
export QWEN_API_KEY="sk-your-api-key-here"

# 或者创建 .env 文件
echo "QWEN_API_KEY=sk-your-api-key-here" > .env
```

**获取 API Key**: https://dashscope.aliyun.com/

### 3. 运行示例

```bash
# 运行营销系统
cargo run --example marketing_generator --features openai-client

# 运行食物分析（需要准备图片文件）
cargo run --example food_analysis_app --features openai-client
```

## 🎯 核心概念

### Agent (智能体)

执行具体任务的基本单元，通过LLM处理输入并生成输出。

```json
{
  "id": "agent_analyzer",
  "type": "agent",
  "config": {
    "name": "analyzer",
    "driver": "qwen",
    "model": "qwen-max",
    "endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1",
    "api_key": "${QWEN_API_KEY}",
    "role": "Analyzer",
    "prompt": "分析用户输入...",
    "intent": "analyze"
  }
}
```

### 工作流 (Workflow)

Agent 和控制流节点的组合，定义任务的执行顺序和逻辑。

```json
{
  "id": "workflow_main",
  "type": "workflow",
  "config": {
    "name": "main_flow",
    "start": "agent_analyzer"
  }
}
```

### 边 (Edges)

连接节点的逻辑，可以是无条件 (always) 或条件 (conditional)。

```json
{
  "from": "agent_classifier",
  "to": "agent_handler",
  "type": "conditional",
  "condition": {
    "type": "state_equals",
    "key": "category",
    "value": "urgent"
  },
  "workflow": "workflow_main"
}
```

### 控制流节点

- **Loop Node** - 循环重试，支持条件判断和最大迭代次数
- **Join Node** - 并行汇总，支持all/any策略
- **Terminal Node** - 流程结束标记

## 🔧 自定义开发

### 创建新应用

1. 创建新的 Rust 文件：`examples/my_app.rs`
2. 创建配置文件：`configs/graph_config_my_app.json`
3. 定义工作流逻辑（nodes + edges）
4. 运行测试

### 配置文件结构

```json
{
  "name": "my_app",
  "version": "1.0",
  "description": "应用描述",
  "nodes": [
    {
      "id": "agent_xxx",
      "type": "agent",
      "config": { /* agent配置 */ },
      "workflow": "workflow_main"
    },
    {
      "id": "node_xxx",
      "type": "join_node|loop_node|terminal_node",
      "config": { /* 节点配置 */ },
      "workflow": "workflow_main"
    },
    {
      "id": "workflow_main",
      "type": "workflow",
      "config": {
        "name": "main_flow",
        "start": "agent_xxx"
      }
    }
  ],
  "edges": [
    {
      "from": "agent_xxx",
      "to": "node_yyy",
      "type": "always|conditional",
      "workflow": "workflow_main"
    }
  ]
}
```

### 示例模板

```rust
use agentflow::state::MemoryStore;
use agentflow::{FlowContext, FlowExecutor, GraphConfig, MessageRole, StructuredMessage};
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置
    let config_json = fs::read_to_string("configs/graph_config_my_app.json")?;
    let graph_config = GraphConfig::from_json(&config_json)?;
    
    // 2. 验证配置
    graph_config.validate()?;
    
    // 3. 加载工作流
    let bundle = graph_config.load_workflow("workflow_main")?;
    let ctx = Arc::new(FlowContext::new(Arc::new(MemoryStore::new())));
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools);
    
    // 4. 创建初始消息
    let initial_message = StructuredMessage::new(serde_json::json!({
        "input": "your input data"
    }))
    .into_agent_message(
        MessageRole::User,
        "user",
        Some("agent_xxx".to_string()),
    )?;
    
    // 5. 执行工作流
    let result = executor.start(Arc::clone(&ctx), initial_message).await?;
    
    // 6. 处理结果
    println!("执行完成！");
    if let Some(msg) = result.last_message {
        println!("结果: {}", msg.content);
    }
    
    Ok(())
}
```

## 📊 应用对比

| 应用 | Agent数量 | 节点类型 | 执行时间 | API调用 | 复杂度 |
|------|----------|---------|---------|---------|--------|
| food_analysis_app | 9 | Loop, Join | ~20-30秒 | 6-9次 | 高 |
| marketing_generator | 13 | Join | ~30-90秒 | 13次 | 高 |

## 💡 最佳实践

1. **API Key管理** - 使用环境变量，不要硬编码
2. **配置驱动** - 所有endpoint、model都在JSON配置中
3. **错误处理** - 捕获并处理所有可能的错误
4. **日志输出** - 添加清晰的进度和状态信息
5. **配置验证** - 运行前使用 `graph_config.validate()` 验证
6. **结果解析** - 使用结构化JSON方式解析LLM输出

## 🐛 故障排查

### 常见错误

**1. API Key 未设置**
```
Error: 环境变量 'QWEN_API_KEY' 未设置
```
**解决**: `export QWEN_API_KEY="your-key"`

**2. 网络连接失败**
```
Error: 请求失败: Connection refused
```
**解决**: 检查网络连接和代理设置

**3. 配置文件错误**
```
Error: Failed to parse graph config
```
**解决**: 检查 JSON 格式和字段完整性

**4. 图片生成403错误**
```
Error: Workspace.AccessDenied
```
**解决**: 
- 确认API Key已开通图片生成服务
- 检查模型名称是否正确（wan2.5-t2i-preview）

### 调试技巧

```bash
# 查看详细日志
RUST_LOG=debug cargo run --example <name> --features openai-client

# 检查配置文件
cat configs/graph_config_<name>.json | jq .

# 验证编译
cargo check --example <name> --features openai-client

# 只编译不运行
cargo build --example <name> --features openai-client
```

## 📚 相关文档

- [完整使用指南](../docs/完整使用指南.md)
- [智能营销系统设计](../docs/智能营销内容生成系统设计.md)
- [食物分析应用设计](../docs/食物识别分析应用设计.md)
- [配置规范-OpenAPI](../docs/配置规范-OpenAPI.md)
- [代码规范和最佳实践](../docs/代码规范和最佳实践.md)

## 🤝 贡献

欢迎贡献新的应用示例！

1. Fork 项目
2. 创建应用分支
3. 添加应用代码和配置文件
4. 更新文档
5. 提交 Pull Request

---

**最后更新**: 2024-11-24  
**AgentFlow 版本**: v2.2.0  
**维护状态**: ✅ 积极维护
