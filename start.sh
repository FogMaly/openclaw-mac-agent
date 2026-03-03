#!/bin/bash

# OpenClaw Mac Agent 快速启动脚本

set -e

AGENT_DIR="$HOME/openclaw-mac-agent"
BINARY="$AGENT_DIR/target/release/openclaw-agent"
CONFIG_DIR="$HOME/.openclaw-agent"
CONFIG_FILE="$CONFIG_DIR/config.json"

echo "🚀 OpenClaw Mac Agent 启动脚本"
echo "================================"

# 检查二进制文件是否存在
if [ ! -f "$BINARY" ]; then
    echo "❌ 未找到编译后的二进制文件"
    echo "请先运行: cd $AGENT_DIR && cargo build --release"
    exit 1
fi

# 检查配置文件
if [ ! -f "$CONFIG_FILE" ]; then
    echo "⚠️  配置文件不存在，将在首次运行时自动创建"
fi

echo "✅ 二进制文件: $BINARY"
echo "✅ 配置目录: $CONFIG_DIR"
echo ""
echo "启动 Agent..."
echo ""

# 运行 Agent
exec "$BINARY"
