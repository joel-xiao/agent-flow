# 更新日志 / Changelog

## [LegalFlow Application] - 2024-11-25

### 🆕 新增应用示例

**LegalFlow - 智能法律案件评估与文书生成系统**
- **完整功能展示**: 展示了 AgentFlow 框架的所有核心功能
  - ✅ 智能案件分析与领域识别
  - ✅ 图驱动条件路由（基于 State 变量）
  - ✅ 多角色专业协作（10+ 专业角色）
  - ✅ 循环质量审核机制（Loop Node）
  - ✅ 并行多语言处理（Join Node）
  - ✅ AI 可视化证据生成
  - ✅ 工具集成（图片生成 + 文件下载）
  - ✅ 状态共享与上下文注入
- **技术亮点**:
  - 使用条件边（Conditional Edges）实现确定性路由，避免 LLM 路由决策的不稳定性
  - 完整的 `extract_to_state` 配置示例，展示状态管理最佳实践
  - 循环节点（Loop Node）实现自动质量审核重试
  - 并行节点（Join Node）实现多语言任务同步
- **文件**:
  - `examples/legal_flow_app.rs` - 应用入口
  - `configs/graph_config_legal_flow.json` - 完整工作流配置
  - `docs/智能法律案件评估与文书生成系统设计.md` - 详细设计文档

## [Core & Examples Update] - 2024-11-25

### ✨ 核心功能增强 (Core Enhancements)

**1. Prompt Context Injection (上下文注入)**
- `PromptBuildingRules` 新增配置项：
  - `include_store_keys`: 允许指定一组 State Store 变量键名，系统会自动提取这些变量并注入到 Agent 的 System Prompt 中。
  - `max_history_items`: 允许配置注入 Prompt 的历史消息数量（默认为 3）。
- 解决了 Agent 之间非直接消息传递（通过 Shared State）的上下文感知问题。

**2. Advanced Field Extraction (高级字段提取)**
- `MessageParser` 增强：支持从 Payload 中提取非字符串类型的字段（自动转换为 JSON 字符串）。
- 解决了复杂对象（如 `product_info`）无法作为 LLM 输入的问题。

**3. Configurable Field Extraction (配置化字段提取)**
- `FieldExtractionRules` 新增 `extract_to_state` 映射配置。
- 支持完全通过 JSON 配置将 LLM 响应字段提取到 State Store，无需修改代码。

### 🚀 示例应用升级 (Example Updates)

**Marketing Content Generator (智能营销内容生成器)**
- **流程重构**：升级为多角色协作的高级工作流。
  - **Product Visualizer**: 专门负责分析产品视觉特征。
  - **Creative Director**: 负责制定 Slogan 和视觉创意概念。
  - **AI Visual Prompter**: 结合视觉特征和创意概念，生成专业的海报 Prompt。
- **效果优化**：修复了“文不对题”和“产品缺失”的问题，能够生成高质量、符合产品特征的营销海报。
- **配置更新**：`configs/graph_config_marketing_generator.json` 全面更新。

### 🛠️ 代码清理与修复 (Cleanup & Fixes)

- **移除废弃代码**：删除了 `ImageProcessor` 模块（设计不合理，功能已移至 Prompt Engineering 层）。
- **修复硬编码**：移除了 Agent 中硬编码的业务字段，完全由配置驱动。
- **修复编译错误**：修复了所有因重构引入的编译问题。

## [最近更新] - 2024-11-24

### 🗑️ 删除过期内容

**过期测试文件**
- 删除 `tests/test_prompt_builder.rs` - 与源代码中的单元测试重复
- 删除 `tests/test_message_parser.rs` - 与源代码中的单元测试重复
- 删除 `tests/test_compile_check.rs` - 临时调试文件

### 📝 文档更新

**修复文档引用**
- 更新 `docs/路由和编排功能说明.md` - 移除对不存在文档的引用，补充自动路由说明

**新增文档索引**
- 新增 `docs/README.md` - 文档中心索引，提供快速导航

### ✅ 验证通过的文档

以下文档已验证与当前代码库完全一致：

**工具系统文档**
- `docs/工具系统架构.md` ✅
- `docs/JSON配置驱动的工具使用.md` ✅
- `docs/内置工具使用说明.md` ✅
- `docs/下载工具配置示例.md` ✅

**核心功能文档**
- `docs/路由和编排功能说明.md` ✅ (已更新)
- `docs/流式输出实现说明.md` ✅
- `docs/配置API_Key指南.md` ✅

**应用示例文档**
- `docs/智能营销内容生成系统设计.md` ✅
- `docs/食物识别分析应用设计.md` ✅
- `docs/完整使用指南.md` ✅

**开发指南**
- `docs/代码规范和最佳实践.md` ✅
- `README.md` ✅

### 📊 文档统计

- **总计文档**: 12 个
- **已验证**: 12 个
- **已更新**: 2 个 (路由和编排功能说明.md, docs/README.md)
- **新增**: 1 个 (docs/README.md)
- **删除过期测试**: 3 个

## 文档更新原因

### 删除过期测试的原因

1. **test_prompt_builder.rs**
   - 源代码文件 `src/flow/services/prompt_builder.rs` 中已包含完整的单元测试
   - tests 目录中的测试与源代码测试重复
   - 测试覆盖范围：`build_system_prompt`, `add_routing_instructions`, `build_system_prompt_with_routing`

2. **test_message_parser.rs**
   - 源代码文件 `src/flow/services/message_parser.rs` 中已包含完整的单元测试
   - tests 目录中的测试与源代码测试重复
   - 测试覆盖范围：`parse_payload`, `extract_steps`, `extract_user_input`

3. **test_compile_check.rs**
   - 这是一个临时调试文件，用于排查编译问题
   - 已完成调试任务，不再需要保留

### 文档更新原则

- ✅ 保持文档与代码同步
- ✅ 删除对不存在文件的引用
- ✅ 提供准确的技术说明
- ✅ 保持文档结构清晰
- ✅ 提供快速导航索引

## 下一步计划

- [ ] 持续保持文档与代码同步
- [ ] 根据新功能添加相应文档
- [ ] 收集用户反馈，改进文档质量
