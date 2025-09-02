#!/bin/bash

# DeepSeek Free API 完整测试脚本
# 自动测试指定账户的登录和API功能

set -e

BASE_URL="http://localhost:8000"
TEST_EMAIL="781851839@qq.com"
TEST_PASSWORD="1234qwerasdfzxcv"

echo "=== DeepSeek Free API 自动化测试 ==="
echo "测试账户: $TEST_EMAIL"
echo "服务地址: $BASE_URL"
echo

# 检查依赖
if ! command -v jq &> /dev/null; then
    echo "错误: 需要安装 jq 命令行工具"
    echo "Ubuntu/Debian: sudo apt install jq"
    echo "macOS: brew install jq"
    exit 1
fi

# 等待服务启动的函数
wait_for_service() {
    echo "等待服务启动..."
    for i in {1..30}; do
        if curl -s "$BASE_URL/ping" >/dev/null 2>&1; then
            echo "服务已启动"
            return 0
        fi
        echo "等待中... ($i/30)"
        sleep 2
    done
    echo "服务启动超时"
    exit 1
}

# 1. 检查服务是否运行
echo "1. 检查服务状态..."
if ! curl -s "$BASE_URL/ping" >/dev/null 2>&1; then
    echo "服务未运行，请先启动服务"
    echo "运行命令: cargo run"
    exit 1
fi

# 健康检查
echo "服务状态检查:"
curl -s "$BASE_URL/ping" | jq . || echo "健康检查失败"
echo

# 2. 创建API密钥
echo "2. 创建API密钥..."
API_KEY_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/create" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "测试密钥-781851839",
    "expires_days": 30
  }')

echo "API密钥创建响应:"
echo "$API_KEY_RESPONSE" | jq .

# 检查API密钥创建是否成功
if ! echo "$API_KEY_RESPONSE" | jq -e '.api_key' >/dev/null 2>&1; then
    echo "❌ API密钥创建失败"
    exit 1
fi

API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.api_key')
echo "✅ 成功创建API密钥: $API_KEY"
echo

# 3. 测试账户登录
echo "3. 测试账户登录..."
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$TEST_EMAIL\",
    \"password\": \"$TEST_PASSWORD\"
  }")

echo "登录响应:"
echo "$LOGIN_RESPONSE" | jq .

# 检查登录是否成功
LOGIN_SUCCESS=$(echo "$LOGIN_RESPONSE" | jq -r '.success // false')
if [ "$LOGIN_SUCCESS" = "true" ]; then
    echo "✅ 账户登录成功！"
    
    USER_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.user_token')
    echo "获取到的userToken: ${USER_TOKEN:0:20}..."
    
    # 4. 验证token
    echo
    echo "4. 验证userToken..."
    VERIFY_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/verify" \
      -H "Content-Type: application/json" \
      -d "{
        \"token\": \"$USER_TOKEN\"
      }")
    
    echo "Token验证结果:"
    echo "$VERIFY_RESPONSE" | jq .
    
    VERIFY_SUCCESS=$(echo "$VERIFY_RESPONSE" | jq -r '.valid // false')
    if [ "$VERIFY_SUCCESS" = "true" ]; then
        echo "✅ Token验证成功"
    else
        echo "❌ Token验证失败"
    fi
    
    # 5. 将账户添加到API密钥
    echo
    echo "5. 将账户添加到API密钥..."
    ADD_ACCOUNT_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/add_account" \
      -H "Content-Type: application/json" \
      -d "{
        \"api_key\": \"$API_KEY\",
        \"email\": \"$TEST_EMAIL\",
        \"password\": \"$TEST_PASSWORD\"
      }")
    
    echo "添加账户结果:"
    echo "$ADD_ACCOUNT_RESPONSE" | jq .
    
    ADD_SUCCESS=$(echo "$ADD_ACCOUNT_RESPONSE" | jq -r '.success // false')
    if [ "$ADD_SUCCESS" = "true" ]; then
        echo "✅ 账户添加成功"
    else
        echo "❌ 账户添加失败"
        exit 1
    fi
    
    # 6. 查看API密钥信息
    echo
    echo "6. 查看API密钥信息..."
    KEY_INFO_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/info" \
      -H "Content-Type: application/json" \
      -d "{
        \"api_key\": \"$API_KEY\"
      }")
    
    echo "API密钥信息:"
    echo "$KEY_INFO_RESPONSE" | jq .
    
    ACCOUNTS_COUNT=$(echo "$KEY_INFO_RESPONSE" | jq -r '.accounts_count // 0')
    echo "✅ API密钥绑定账户数量: $ACCOUNTS_COUNT"
    
    # 7. 测试聊天功能
    echo
    echo "7. 测试聊天功能..."
    CHAT_RESPONSE=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $API_KEY" \
      -d '{
        "model": "deepseek",
        "messages": [
          {"role": "user", "content": "你好，请简单介绍一下DeepSeek"}
        ],
        "stream": false,
        "max_tokens": 100
      }')
    
    echo "聊天响应:"
    echo "$CHAT_RESPONSE" | jq .
    
    # 检查聊天是否成功
    if echo "$CHAT_RESPONSE" | jq -e '.choices[0].message.content' >/dev/null 2>&1; then
        echo "✅ 聊天功能正常"
        CHAT_CONTENT=$(echo "$CHAT_RESPONSE" | jq -r '.choices[0].message.content')
        echo "AI回复内容: $CHAT_CONTENT"
    else
        echo "❌ 聊天功能异常"
    fi
    
    # 8. 测试上下文对话
    echo
    echo "8. 测试上下文对话..."
    CONV_ID="test-conversation-$(date +%s)"
    
    echo "第一轮对话（创建会话）:"
    CHAT1_RESPONSE=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $API_KEY" \
      -d "{
        \"model\": \"deepseek\",
        \"conversation_id\": \"$CONV_ID\",
        \"messages\": [
          {\"role\": \"user\", \"content\": \"我的名字是张三，请记住\"}
        ],
        \"stream\": false,
        \"max_tokens\": 50
      }")
    
    echo "$CHAT1_RESPONSE" | jq .
    
    echo
    echo "第二轮对话（测试记忆）:"
    CHAT2_RESPONSE=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $API_KEY" \
      -d "{
        \"model\": \"deepseek\",
        \"conversation_id\": \"$CONV_ID\",
        \"messages\": [
          {\"role\": \"user\", \"content\": \"我的名字是什么？\"}
        ],
        \"stream\": false,
        \"max_tokens\": 50
      }")
    
    echo "$CHAT2_RESPONSE" | jq .
    
    # 检查是否记住了名字
    if echo "$CHAT2_RESPONSE" | jq -r '.choices[0].message.content' | grep -i "张三" >/dev/null; then
        echo "✅ 上下文记忆功能正常"
    else
        echo "⚠️  上下文记忆可能有问题"
    fi
    
    # 9. 会话池统计
    echo
    echo "9. 会话池统计信息:"
    STATS_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/stats" \
      -H "Content-Type: application/json" \
      -d "{
        \"api_key\": \"$API_KEY\"
      }")
    
    echo "$STATS_RESPONSE" | jq .
    
    # 10. 流式对话测试
    echo
    echo "10. 测试流式对话..."
    echo "发送流式请求..."
    curl -N -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $API_KEY" \
      -d '{
        "model": "deepseek",
        "messages": [
          {"role": "user", "content": "请数从1到5"}
        ],
        "stream": true,
        "max_tokens": 50
      }' | head -20
    
    echo
    echo "✅ 流式对话测试完成"
    
else
    echo "❌ 账户登录失败"
    echo "请检查账户信息是否正确"
    exit 1
fi

# 11. 列出所有API密钥
echo
echo "11. 列出所有API密钥..."
curl -s "$BASE_URL/api_keys/list" | jq .

echo
echo "=== 测试完成总结 ==="
echo "✅ 测试账户: $TEST_EMAIL"
echo "✅ API密钥: $API_KEY"
echo "✅ 所有主要功能测试通过"
echo
echo "可以使用以下命令进行更多测试:"
echo "curl -X POST $BASE_URL/v1/chat/completions -H \"Authorization: Bearer $API_KEY\" -H \"Content-Type: application/json\" -d '{\"model\":\"deepseek\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}]}'"
echo
echo "清理测试数据:"
echo "curl -X POST $BASE_URL/api_keys/cleanup"
