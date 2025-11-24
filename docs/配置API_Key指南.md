# API Key 配置指南

## 安全最佳实践

AgentFlow 现在支持通过环境变量管理 API Key，避免在配置文件中硬编码敏感信息。

## 配置方法

### 方法 1：使用环境变量引用（推荐）

在配置文件中使用 `${ENV_VAR_NAME}` 格式引用环境变量：

```json
{
  "id": "agent_food_identifier",
  "type": "agent",
  "config": {
    "name": "food_identifier",
    "driver": "qwen",
    "api_key": "${QWEN_API_KEY}",
    "model": "qwen-vl-max",
    ...
  }
}
```

然后在环境中设置：

```bash
export QWEN_API_KEY="sk-your-actual-api-key"
```

### 方法 2：使用 .env 文件（推荐）

1. 复制 `.env.example` 为 `.env`：
   ```bash
   cp .env.example .env
   ```

2. 编辑 `.env` 文件，填入实际的 API Key：
   ```
   QWEN_API_KEY=sk-your-actual-api-key
   MOONSHOT_API_KEY=your-moonshot-key
   BIGMODEL_API_KEY=your-bigmodel-key
   ```

3. 在代码中加载 .env 文件（如果使用 dotenv）

### 方法 3：省略 api_key 字段

如果配置中完全省略 `api_key` 字段，系统会自动从对应 driver 的默认环境变量读取：

- `qwen` driver → `QWEN_API_KEY`
- `moonshot` driver → `MOONSHOT_API_KEY`
- `bigmodel` driver → `BIGMODEL_API_KEY`

```json
{
  "id": "agent_food_identifier",
  "type": "agent",
  "config": {
    "name": "food_identifier",
    "driver": "qwen",
    "model": "qwen-vl-max"
    // api_key 省略，自动从 QWEN_API_KEY 环境变量读取
  }
}
```

## 环境变量名称

| Driver | 默认环境变量 |
|--------|------------|
| qwen | QWEN_API_KEY |
| moonshot | MOONSHOT_API_KEY |
| bigmodel | BIGMODEL_API_KEY |

## 迁移现有配置

如果你的配置文件中已经硬编码了 API Key，建议：

1. 将 API Key 移到环境变量或 .env 文件
2. 在配置文件中替换为 `"${ENV_VAR_NAME}"` 引用
3. 确保 `.env` 文件在 `.gitignore` 中（已自动添加）

## 安全提示

✅ **应该做：**
- 使用环境变量存储 API Key
- 使用 `.env` 文件（并加入 .gitignore）
- 在配置文件中使用 `${VAR_NAME}` 引用

❌ **不应该做：**
- 在配置文件中硬编码 API Key
- 将包含 API Key 的 .env 文件提交到 Git
- 在日志或错误消息中打印完整的 API Key

## 错误排查

### "环境变量未设置"错误

如果遇到类似错误：
```
环境变量 'QWEN_API_KEY' 未设置。请在 .env 文件中设置或通过环境变量传递。
```

解决方法：
1. 检查是否设置了对应的环境变量
2. 检查 .env 文件是否存在且格式正确
3. 确认环境变量名称与配置中的引用一致

### API Key 格式问题

系统会自动识别以下格式：
- `${VAR_NAME}` → 从环境变量读取
- `your_api_key_here` → 视为占位符，从环境变量读取
- `sk-xxxxx...` （长度 > 20）→ 视为实际的 API Key

## 示例

完整的配置示例：

```json
{
  "name": "my_workflow",
  "nodes": [
    {
      "id": "service_qwen",
      "type": "service",
      "config": {
        "name": "qwen",
        "service_type": "llm",
        "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "api_key": "${QWEN_API_KEY}"
      }
    },
    {
      "id": "agent_analyzer",
      "type": "agent",
      "config": {
        "name": "analyzer",
        "driver": "qwen",
        "model": "qwen-max",
        "api_key": "${QWEN_API_KEY}",
        // 或者省略 api_key，自动从 QWEN_API_KEY 读取
        ...
      }
    }
  ]
}
```

对应的 .env 文件：

```bash
QWEN_API_KEY=sk-your-actual-qwen-api-key-here
```

