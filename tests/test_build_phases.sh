#!/bin/bash
# 分阶段编译测试脚本 - 排查编译卡死问题

set -e

echo "=== 编译阶段测试 ==="
echo ""

echo "1. 检查 Cargo 配置..."
cargo --version
echo "✅ Cargo 可用"
echo ""

echo "2. 检查依赖..."
cargo tree --depth 1 2>&1 | head -20
echo "✅ 依赖检查完成"
echo ""

echo "3. 语法检查 (cargo check)..."
timeout 60 cargo check --lib 2>&1 | tail -10
echo "✅ 语法检查完成"
echo ""

echo "4. 编译库..."
timeout 120 cargo build --lib 2>&1 | tail -10
echo "✅ 库编译完成"
echo ""

echo "5. 编译示例..."
timeout 120 cargo build --example food_analysis_app --features openai-client 2>&1 | tail -10
echo "✅ 示例编译完成"
echo ""

echo "=== 所有编译阶段测试通过 ==="

