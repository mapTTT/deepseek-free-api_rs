#!/bin/bash

# 测试API密钥管理系统的脚本

BASE_URL="http://localhost:8000"

echo "=== DeepSeek Free API 登录和API密钥管理测试 ==="
echo

# 1. 健康检查
echo "1. 检查服务状态..."
curl -s "$BASE_URL/ping" | jq . || echo "服务未启动"
echo

# 2. 创建API密钥
echo "2. 创建API密钥..."
API_KEY_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/create" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "测试密钥",
    "expires_days": 30
  }')

echo "$API_KEY_RESPONSE" | jq .

# 提取API密钥
API_KEY=$(echo "$API_KEY_RESPONSE" | jq -r '.api_key')
echo "创建的API密钥: $API_KEY"
echo

# 3. 测试直接登录（需要真实的邮箱和密码）
echo "3. 测试直接登录（请输入DeepSeek账户信息）..."
echo -n "邮箱: "
read EMAIL
echo -n "密码: "
read -s PASSWORD
echo

LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$EMAIL\",
    \"password\": \"$PASSWORD\"
  }")

echo "登录响应:"
echo "$LOGIN_RESPONSE" | jq .

# 检查登录是否成功
LOGIN_SUCCESS=$(echo "$LOGIN_RESPONSE" | jq -r '.success')
if [ "$LOGIN_SUCCESS" = "true" ]; then
    echo "✓ 登录成功！"
    
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
    
    # 5. 将账户添加到API密钥
    echo
    echo "5. 将账户添加到API密钥..."
    ADD_ACCOUNT_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/add_account" \
      -H "Content-Type: application/json" \
      -d "{
        \"api_key\": \"$API_KEY\",
        \"email\": \"$EMAIL\",
        \"password\": \"$PASSWORD\"
      }")
    
    echo "添加账户结果:"
    echo "$ADD_ACCOUNT_RESPONSE" | jq .
    
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
    
    # 7. 使用API密钥测试聊天
    echo
    echo "7. 使用API密钥测试聊天..."
    CHAT_RESPONSE=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $API_KEY" \
      -d '{
        "model": "deepseek",
        "messages": [
          {"role": "user", "content": "你好，请简单介绍一下自己"}
        ],
        "stream": false,
        "max_tokens": 100
      }')
    
    echo "聊天响应:"
    echo "$CHAT_RESPONSE" | jq .
    
    # 7.1 测试上下文对话
    echo
    echo "7.1 测试上下文对话..."
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
    echo "第二轮对话（使用相同conversation_id）:"
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
    
    # 7.2 查看会话池统计
    echo
    echo "7.2 会话池统计信息:"
    STATS_RESPONSE=$(curl -s -X POST "$BASE_URL/api_keys/stats" \
      -H "Content-Type: application/json" \
      -d "{
        \"api_key\": \"$API_KEY\"
      }")
    
    echo "$STATS_RESPONSE" | jq .
    
else
    echo "✗ 登录失败"
fi

# 8. 列出所有API密钥
echo
echo "8. 列出所有API密钥..."
curl -s "$BASE_URL/api_keys/list" | jq .

echo
echo "=== 测试完成 ==="
echo "如需清理测试数据，可以调用: curl -X POST $BASE_URL/api_keys/cleanup"
