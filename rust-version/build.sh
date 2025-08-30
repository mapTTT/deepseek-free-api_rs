#!/bin/bash

# 设置环境变量
export PATH="$HOME/.cargo/bin:$PATH"
export RUST_LOG=info

# 进入项目目录
cd /workspaces/deepseek-free-api_rs/rust-version

# 检查编译
echo "检查编译..."
cargo check

if [ $? -eq 0 ]; then
    echo "编译检查通过!"
    
    # 构建项目
    echo "构建项目..."
    cargo build --release
    
    if [ $? -eq 0 ]; then
        echo "构建成功!"
        echo "可执行文件位置: target/release/deepseek-free-api"
        
        # 显示使用说明
        echo ""
        echo "使用说明："
        echo "1. 设置环境变量:"
        echo "   export DEEP_SEEK_CHAT_AUTHORIZATION=your_token"
        echo ""
        echo "2. 运行服务器:"
        echo "   ./target/release/deepseek-free-api"
        echo ""
        echo "3. 测试API:"
        echo "   curl -X POST http://localhost:8000/v1/chat/completions \\"
        echo "     -H 'Content-Type: application/json' \\"
        echo "     -H 'Authorization: Bearer your_token' \\"
        echo "     -d '{\"model\":\"deepseek\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}]}'"
        
    else
        echo "构建失败!"
        exit 1
    fi
else
    echo "编译检查失败!"
    exit 1
fi
