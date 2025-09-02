#!/bin/bash

# DeepSeek API 测试脚本

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 服务器配置
BASE_URL="http://localhost:8000"

echo -e "${BLUE}=== DeepSeek Free API 测试 ===${NC}"
echo

# 1. 测试健康检查
echo -e "${YELLOW}1. 测试健康检查...${NC}"
response=$(curl -s -w "%{http_code}" -o /tmp/health_response.json "${BASE_URL}/health")
http_code="${response: -3}"

if [ "$http_code" = "200" ]; then
    echo -e "${GREEN}✓ 健康检查成功${NC}"
    cat /tmp/health_response.json | jq '.' 2>/dev/null || cat /tmp/health_response.json
else
    echo -e "${RED}✗ 健康检查失败 (HTTP $http_code)${NC}"
    cat /tmp/health_response.json 2>/dev/null
fi
echo

# 2. 测试模型列表
echo -e "${YELLOW}2. 测试模型列表...${NC}"
response=$(curl -s -w "%{http_code}" -o /tmp/models_response.json "${BASE_URL}/v1/models")
http_code="${response: -3}"

if [ "$http_code" = "200" ]; then
    echo -e "${GREEN}✓ 获取模型列表成功${NC}"
    cat /tmp/models_response.json | jq '.data[] | .id' 2>/dev/null || cat /tmp/models_response.json
else
    echo -e "${RED}✗ 获取模型列表失败 (HTTP $http_code)${NC}"
    cat /tmp/models_response.json 2>/dev/null
fi
echo

# 3. 测试登录 (使用提供的账户)
echo -e "${YELLOW}3. 测试登录...${NC}"
login_response=$(curl -s -w "%{http_code}" -o /tmp/login_response.json \
    -X POST "${BASE_URL}/login" \
    -H "Content-Type: application/json" \
    -d '{
        "email": "781851839@qq.com",
        "password": "1234qwerasdfzxcv"
    }')
http_code="${login_response: -3}"

if [ "$http_code" = "200" ]; then
    echo -e "${GREEN}✓ 登录成功${NC}"
    user_token=$(cat /tmp/login_response.json | jq -r '.user_token' 2>/dev/null)
    if [ "$user_token" != "null" ] && [ "$user_token" != "" ]; then
        echo "用户令牌: ${user_token:0:20}..."
        echo
        
        # 4. 使用用户令牌测试聊天
        echo -e "${YELLOW}4. 测试聊天补全 (使用用户令牌)...${NC}"
        chat_response=$(curl -s -w "%{http_code}" -o /tmp/chat_response.json \
            -X POST "${BASE_URL}/v1/chat/completions" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $user_token" \
            -d '{
                "model": "deepseek",
                "messages": [
                    {
                        "role": "user",
                        "content": "你好，请简单介绍一下你自己。"
                    }
                ],
                "stream": false
            }')
        http_code="${chat_response: -3}"
        
        if [ "$http_code" = "200" ]; then
            echo -e "${GREEN}✓ 聊天补全成功${NC}"
            cat /tmp/chat_response.json | jq '.choices[0].message.content' 2>/dev/null || cat /tmp/chat_response.json
        else
            echo -e "${RED}✗ 聊天补全失败 (HTTP $http_code)${NC}"
            cat /tmp/chat_response.json 2>/dev/null
        fi
        echo
        
        # 5. 测试流式聊天
        echo -e "${YELLOW}5. 测试流式聊天...${NC}"
        echo "请求流式响应..."
        curl -s -X POST "${BASE_URL}/v1/chat/completions" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $user_token" \
            -d '{
                "model": "deepseek",
                "messages": [
                    {
                        "role": "user", 
                        "content": "请用一句话介绍什么是人工智能。"
                    }
                ],
                "stream": true
            }' | head -n 10
        echo -e "\n${GREEN}✓ 流式响应测试完成${NC}"
    else
        echo -e "${RED}✗ 未能获取有效的用户令牌${NC}"
    fi
else
    echo -e "${RED}✗ 登录失败 (HTTP $http_code)${NC}"
    cat /tmp/login_response.json 2>/dev/null
fi
echo

# 6. 测试API密钥管理 (如果有的话)
echo -e "${YELLOW}6. 测试API密钥创建...${NC}"
api_key_response=$(curl -s -w "%{http_code}" -o /tmp/api_key_response.json \
    -X POST "${BASE_URL}/api-keys" \
    -H "Content-Type: application/json" \
    -d '{
        "accounts": [
            {
                "email": "781851839@qq.com",
                "password": "1234qwerasdfzxcv"
            }
        ]
    }')
http_code="${api_key_response: -3}"

if [ "$http_code" = "200" ]; then
    echo -e "${GREEN}✓ API密钥创建成功${NC}"
    api_key=$(cat /tmp/api_key_response.json | jq -r '.api_key' 2>/dev/null)
    if [ "$api_key" != "null" ] && [ "$api_key" != "" ]; then
        echo "API密钥: $api_key"
        echo
        
        # 7. 使用API密钥测试聊天
        echo -e "${YELLOW}7. 测试聊天补全 (使用API密钥)...${NC}"
        api_chat_response=$(curl -s -w "%{http_code}" -o /tmp/api_chat_response.json \
            -X POST "${BASE_URL}/v1/chat/completions" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $api_key" \
            -d '{
                "model": "deepseek",
                "messages": [
                    {
                        "role": "user",
                        "content": "测试API密钥是否工作正常，请回复收到。"
                    }
                ],
                "stream": false
            }')
        http_code="${api_chat_response: -3}"
        
        if [ "$http_code" = "200" ]; then
            echo -e "${GREEN}✓ API密钥聊天补全成功${NC}"
            cat /tmp/api_chat_response.json | jq '.choices[0].message.content' 2>/dev/null || cat /tmp/api_chat_response.json
        else
            echo -e "${RED}✗ API密钥聊天补全失败 (HTTP $http_code)${NC}"
            cat /tmp/api_chat_response.json 2>/dev/null
        fi
    else
        echo -e "${RED}✗ 未能获取有效的API密钥${NC}"
    fi
else
    echo -e "${RED}✗ API密钥创建失败 (HTTP $http_code)${NC}"
    cat /tmp/api_key_response.json 2>/dev/null
fi

echo
echo -e "${BLUE}=== 测试完成 ===${NC}"

# 清理临时文件
rm -f /tmp/health_response.json /tmp/models_response.json /tmp/login_response.json 
rm -f /tmp/chat_response.json /tmp/api_key_response.json /tmp/api_chat_response.json
