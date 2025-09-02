#!/bin/bash

echo "=== DeepSeek Free API 启动脚本 ==="

# 检查依赖
if ! command -v cargo &> /dev/null; then
    echo "错误: 需要安装 Rust 和 Cargo"
    echo "请访问 https://rustup.rs/ 安装 Rust"
    exit 1
fi

# 检查 .env 文件
if [ ! -f ".env" ]; then
    echo "警告: .env 文件不存在，将使用默认配置"
fi

echo "构建项目..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "构建成功，启动服务..."
    echo "服务将在 http://localhost:8000 启动"
    echo "按 Ctrl+C 停止服务"
    echo
    cargo run --release
else
    echo "构建失败，请检查错误信息"
    exit 1
fi
