# JSON 配置驱动的工具使用

## 概述

AgentFlow 的工具系统完全支持 **JSON 配置驱动**，工具参数直接在 graph JSON 中定义，无需在代码中硬编码。

## 配置方式对比

### ❌ 旧方式（代码硬编码）

```rust
// 在代码中硬编码参数
let download_pipeline = ToolPipeline::new(
    "download_file",
    ToolStrategy::Sequential(vec![
        ToolStep::new("downloader", serde_json::json!({
            "save_dir": "generated_images",      // 硬编码
            "filename_prefix": "marketing"       // 硬编码
        }))
    ])
);
```

**缺点**：修改参数需要重新编译代码

### ✅ 新方式（JSON 配置）

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "params": {
      "save_dir": "generated_images",
      "filename_prefix": "marketing"
    }
  }
}
```

```rust
// 代码中只注册空 pipeline
let download_pipeline = ToolPipeline::new(
    "download_file",
    ToolStrategy::Sequential(vec![
        ToolStep::new("downloader", serde_json::json!({}))  // 空参数
    ])
);
```

**优点**：修改参数只需修改 JSON 文件，无需重新编译

## 完整示例

### 1. JSON 配置

```json
{
  "name": "marketing_content_generator",
  "nodes": [
    {
      "id": "agent_image_generator",
      "type": "agent",
      "config": {
        "name": "image_generator",
        "model": "wan2.5-t2i-preview",
        "endpoint": "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis"
      }
    },
    {
      "id": "tool_downloader",
      "type": "tool_node",
      "config": {
        "pipeline": "download_file",
        "params": {
          "save_dir": "output/marketing/images",
          "filename_prefix": "campaign"
        }
      }
    },
    {
      "id": "agent_result_summarizer",
      "type": "agent",
      "config": {
        "name": "result_summarizer",
        "model": "qwen-max"
      }
    }
  ],
  "edges": [
    {
      "from": "agent_image_generator",
      "to": "tool_downloader",
      "type": "always"
    },
    {
      "from": "tool_downloader",
      "to": "agent_result_summarizer",
      "type": "always"
    }
  ]
}
```

### 2. Rust 代码

```rust
use agentflow::tools::{ToolOrchestrator, ToolPipeline, ToolStep, ToolStrategy};
use agentflow::{FlowExecutor, GraphConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置
    let config_json = fs::read_to_string("configs/graph_config.json")?;
    let graph_config = GraphConfig::from_json(&config_json)?;
    let bundle = graph_config.load_workflow(workflow_id)?;
    
    // 2. 创建 ToolOrchestrator（参数从 JSON 读取）
    let mut orchestrator = ToolOrchestrator::new(bundle.tools.clone());
    
    // 注册空 pipeline（参数从 JSON 的 tool_node.config.params 读取）
    orchestrator.register_pipeline(ToolPipeline::new(
        "download_file",
        ToolStrategy::Sequential(vec![
            ToolStep::new("downloader", serde_json::json!({}))
        ])
    ))?;
    
    // 3. 执行工作流
    let executor = FlowExecutor::new(bundle.flow, bundle.agents, bundle.tools)
        .with_tool_orchestrator(Arc::new(orchestrator));
    
    let result = executor.start(ctx, initial_message).await?;
    
    Ok(())
}
```

## 多种配置场景

### 场景 1：默认配置（无参数）

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file"
  }
}
```

**结果**：使用工具的默认参数
- 保存目录：`downloads/`
- 文件名前缀：`file`

### 场景 2：自定义保存目录

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "params": {
      "save_dir": "output/images"
    }
  }
}
```

### 场景 3：自定义文件名前缀

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "params": {
      "filename_prefix": "marketing"
    }
  }
}
```

### 场景 4：完整自定义

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "params": {
      "save_dir": "storage/campaigns/2024",
      "filename_prefix": "campaign",
      "url": "https://example.com/image.png"  // 可选：手动指定 URL
    }
  }
}
```

### 场景 5：多个下载节点，不同配置

```json
{
  "nodes": [
    {
      "id": "tool_download_marketing",
      "type": "tool_node",
      "config": {
        "pipeline": "download_file",
        "params": {
          "save_dir": "output/marketing",
          "filename_prefix": "marketing"
        }
      }
    },
    {
      "id": "tool_download_product",
      "type": "tool_node",
      "config": {
        "pipeline": "download_file",
        "params": {
          "save_dir": "output/products",
          "filename_prefix": "product"
        }
      }
    }
  ]
}
```

## 参数合并机制

框架会自动合并以下参数：

1. **JSON 配置的 params**（优先级最高）
2. **Pipeline 中的 step.input**（优先级中）
3. **工具的默认值**（优先级最低）

### 示例：参数优先级

```json
// JSON 配置
{
  "params": {
    "save_dir": "output/images"  // 优先级 1
  }
}
```

```rust
// Pipeline 定义
ToolStep::new("downloader", serde_json::json!({
    "save_dir": "downloads",          // 优先级 2（被覆盖）
    "filename_prefix": "file"         // 优先级 2（生效）
}))
```

```rust
// 工具默认值
impl DownloaderTool {
    fn default_save_dir() -> &'static str {
        "downloads"  // 优先级 3（被覆盖）
    }
}
```

**最终参数**：
```json
{
  "save_dir": "output/images",     // 来自 JSON
  "filename_prefix": "file"        // 来自 Pipeline
}
```

## 环境变量支持

可以在 JSON 中使用环境变量：

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "params": {
      "save_dir": "${DOWNLOAD_DIR}",
      "filename_prefix": "${PROJECT_NAME}"
    }
  }
}
```

```bash
export DOWNLOAD_DIR="/Users/username/Downloads"
export PROJECT_NAME="my_campaign"
cargo run --example marketing_generator
```

## 实现原理

### 1. JSON 配置解析

```rust
// src/flow/config/graph.rs
#[derive(Debug, Deserialize, Clone)]
pub enum GraphNode {
    Tool {
        name: String,
        pipeline: String,
        #[serde(default)]
        params: Option<serde_json::Value>,  // ✅ 支持 params
    },
    // ...
}
```

### 2. 参数传递

```rust
// src/runtime/handlers.rs
pub async fn handle_tool_node(
    tool_node: &ToolNode,
    // ...
) -> Result<TaskResult> {
    // 从 tool_node 读取参数（JSON 配置）
    let params = tool_node.params.clone()
        .unwrap_or_else(|| serde_json::json!({}));

    // 传递参数给 orchestrator
    let message = orchestrator
        .execute_pipeline_with_params(&tool_node.pipeline, params, ctx)
        .await?;
    
    // ...
}
```

### 3. 参数合并

```rust
// src/tools/orchestrator.rs
pub async fn execute_strategy_with_params(
    &self,
    strategy: &ToolStrategy,
    params: Value,
    ctx: &FlowContext,
) -> Result<AgentMessage> {
    match strategy {
        ToolStrategy::Sequential(steps) => {
            for step in steps {
                // 合并 step 的 input 和外部传入的 params
                let mut merged_input = step.input.clone();
                if let Some(obj) = merged_input.as_object_mut() {
                    if let Some(params_obj) = params.as_object() {
                        for (k, v) in params_obj {
                            obj.entry(k.clone()).or_insert(v.clone());  // JSON params 优先
                        }
                    }
                }
                
                let merged_step = ToolStep {
                    input: merged_input,
                    ..step.clone()
                };
                self.execute_step(&merged_step, ctx).await?;
            }
            // ...
        }
    }
}
```

## 优势

### 1. ✅ 配置与代码分离
- 修改参数无需重新编译
- 便于不同环境使用不同配置

### 2. ✅ 可维护性
- 所有配置集中在 JSON 文件
- 易于版本控制和团队协作

### 3. ✅ 灵活性
- 支持多个工具节点使用不同参数
- 支持环境变量
- 支持参数合并机制

### 4. ✅ 一致性
- 与 agent 节点的配置方式一致
- 遵循"配置驱动"的设计理念

## 最佳实践

### 1. 使用相对路径

✅ **推荐**：
```json
{
  "params": {
    "save_dir": "output/images"
  }
}
```

❌ **不推荐**：
```json
{
  "params": {
    "save_dir": "/Users/specific_user/Downloads"
  }
}
```

### 2. 有意义的命名

```json
{
  "id": "tool_download_marketing_images",  // 清晰的节点 ID
  "config": {
    "params": {
      "save_dir": "output/marketing/campaigns",
      "filename_prefix": "campaign_image"
    }
  }
}
```

### 3. 按环境组织配置

```bash
configs/
├── graph_config_dev.json      # 开发环境
├── graph_config_staging.json  # 测试环境
└── graph_config_prod.json     # 生产环境
```

```json
// dev
{
  "params": {
    "save_dir": "dev_output"
  }
}

// prod
{
  "params": {
    "save_dir": "/var/app/production/output"
  }
}
```

### 4. 使用注释（JSON5）

虽然标准 JSON 不支持注释，但可以使用 JSON5 或在 `description` 字段中说明：

```json
{
  "id": "tool_downloader",
  "type": "tool_node",
  "config": {
    "pipeline": "download_file",
    "description": "下载 AI 生成的营销图片到本地",
    "params": {
      "save_dir": "output/marketing",
      "filename_prefix": "marketing"
    }
  }
}
```

## 总结

✅ **完全 JSON 驱动**：所有工具参数在 JSON 中配置  
✅ **零代码修改**：修改参数无需重新编译  
✅ **参数合并**：JSON params > Pipeline input > 默认值  
✅ **环境变量**：支持 `${VAR_NAME}` 语法  
✅ **多节点支持**：不同节点可使用不同参数  
✅ **一致性**：与 agent 配置方式保持一致  

**AgentFlow 完全实现了配置驱动的工具系统！** 🎉

