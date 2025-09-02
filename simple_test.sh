#!/bin/bash

# 简单的DeepSeek API测试脚本
# 测试基本功能，不依赖真实的DeepSeek登录

BASE_URL="http://localhost:8000"

echo "=== DeepSeek Free API 基础功能测试 ==="
echo

# 1. 健康检查
echo "1. 健康检查..."
curl -s "$BASE_URL/ping" | jq .
echo

# 2. 模型列表
echo "2. 获取模型列表..."
curl -s "$BASE_URL/v1/models" | jq '.data[].id'
echo

# 3. API密钥创建
echo "3. 创建API密钥..."
api_key_response=$(curl -s -X POST "$BASE_URL/api_keys/create" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "测试密钥-演示",
    "expires_days": 7
  }')

echo "$api_key_response" | jq .
api_key=$(echo "$api_key_response" | jq -r '.api_key')
echo "创建的API密钥: $api_key"
echo

# 4. 查看API密钥信息
echo "4. 查看API密钥信息..."
curl -s -X POST "$BASE_URL/api_keys/info" \
  -H "Content-Type: application/json" \
  -d "{\"api_key\": \"$api_key\"}" | jq .
echo

# 5. 列出所有API密钥
echo "5. 列出所有API密钥..."
curl -s "$BASE_URL/api_keys/list" | jq .
echo

# 6. 测试聊天补全 (无有效token，预期失败)
echo "6. 测试聊天补全 (无有效token，演示错误处理)..."
chat_response=$(curl -s -w "%{http_code}" -o /tmp/chat_demo.json \
  -X POST "$BASE_URL/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $api_key" \
  -d '{
    "model": "deepseek",
    "messages": [
      {"role": "user", "content": "你好"}
    ],
    "stream": false
  }')

http_code="${chat_response: -3}"
echo "HTTP 状态码: $http_code"
cat /tmp/chat_demo.json | jq . 2>/dev/null || cat /tmp/chat_demo.json
echo

# 7. 使用环境变量中的token测试聊天 (如果设置了DEEPSEEK_TOKEN)
if [ ! -z "$DEEPSEEK_TOKEN" ]; then
    echo "7. 使用环境变量token测试聊天..."
    curl -s -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $DEEPSEEK_TOKEN" \
      -d '{
        "model": "deepseek",
        "messages": [
          {"role": "user", "content": "请简单介绍一下你自己"}
        ],
        "stream": false
      }' | jq .
else
    echo "7. 跳过真实聊天测试 (需要设置 DEEPSEEK_TOKEN 环境变量)"
    echo "   如果你有有效的DeepSeek userToken，可以这样测试："
    echo "   export DEEPSEEK_TOKEN='your_user_token_here'"
    echo "   然后重新运行此脚本"
fi
echo

echo "=== 功能说明 ==="
echo "1. API服务器正常运行在 $BASE_URL"
echo "2. 支持OpenAI兼容的聊天API: /v1/chat/completions"
echo "3. 支持模型列表API: /v1/models"
echo "4. 支持API密钥管理功能"
echo "5. 要使用真实的聊天功能，需要："
echo "   - 有效的DeepSeek用户令牌"
echo "   - 或者成功添加DeepSeek账户到API密钥"
echo

echo "=== 使用示例 ==="
echo "# 如果你有有效的userToken，可以直接使用:"
echo "curl -X POST $BASE_URL/v1/chat/completions \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -H 'Authorization: Bearer YOUR_USER_TOKEN' \\"
echo "  -d '{\"model\":\"deepseek\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}]}'"
echo

echo "# 或者使用API密钥 (需要先添加账户):"
echo "curl -X POST $BASE_URL/v1/chat/completions \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -H 'Authorization: Bearer $api_key' \\"
echo "  -d '{\"model\":\"deepseek\",\"messages\":[{\"role\":\"user\",\"content\":\"你好\"}]}'"
echo

# 清理
rm -f /tmp/chat_demo.json

echo "=== 测试完成 ==="
