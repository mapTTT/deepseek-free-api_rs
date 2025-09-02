# DeepSeek Free API (Rust版本) - 登录版

## 功能特点

- ✅ **自动登录**：使用用户名/密码自动登录DeepSeek获取userToken
- ✅ **API密钥管理**：支持多账户轮换的API密钥系统
- ✅ **OpenAI兼容**：完全兼容OpenAI的聊天接口
- ✅ **流式响应**：支持Server-Sent Events流式输出
- ✅ **多模型支持**：支持所有DeepSeek模型
- ✅ **自动挑战**：处理POW挑战验证
- ✅ **Docker部署**：完整的容器化部署方案

## 快速开始

### 1. 构建和运行

```bash
cd rust-version
./build.sh
cargo run
```

服务将在 `http://localhost:3000` 启动。

### 2. 使用方式

#### 方式一：API密钥管理（推荐）

1. **创建API密钥**
```bash
curl -X POST http://localhost:3000/api_keys/create \
  -H "Content-Type: application/json" \
  -d '{
    "name": "我的API密钥",
    "expires_days": 30
  }'
```

响应示例：
```json
{
  "api_key": "dsk-abc123def456...",
  "name": "我的API密钥",
  "created_at": 1703123456,
  "expires_at": 1705715456
}
```

2. **添加DeepSeek账户**
```bash
curl -X POST http://localhost:3000/api_keys/add_account \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "dsk-abc123def456...",
    "email": "your-email@example.com",
    "password": "your-password"
  }'
```

系统会自动：
- 使用提供的用户名密码登录DeepSeek
- 提取userToken
- 将userToken关联到API密钥

3. **使用API密钥进行聊天**
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dsk-abc123def456..." \
  -d '{
    "model": "deepseek",
    "messages": [
      {"role": "user", "content": "你好"}
    ]
  }'
```

#### 方式二：直接使用userToken

如果你已经有userToken，可以直接使用：

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your-user-token>" \
  -d '{
    "model": "deepseek",
    "messages": [
      {"role": "user", "content": "你好"}
    ]
  }'
```

### 3. 管理API

#### 查看API密钥信息
```bash
curl -X POST http://localhost:3000/api_keys/info \
  -H "Content-Type: application/json" \
  -d '{"api_key": "dsk-abc123def456..."}'
```

#### 列出所有API密钥
```bash
curl http://localhost:3000/api_keys/list
```

#### 停用API密钥
```bash
curl -X POST http://localhost:3000/api_keys/deactivate \
  -H "Content-Type: application/json" \
  -d '{"api_key": "dsk-abc123def456..."}'
```

#### 清理过期密钥
```bash
curl -X POST http://localhost:3000/api_keys/cleanup
```

### 4. 调试接口

#### 直接登录获取userToken
```bash
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "your-email@example.com",
    "password": "your-password"
  }'
```

#### 验证userToken
```bash
curl -X POST http://localhost:3000/auth/verify \
  -H "Content-Type: application/json" \
  -d '{"token": "your-user-token"}'
```

## 支持的模型

- `deepseek` - 基础聊天模型
- `deepseek-search` - 带搜索功能
- `deepseek-think` - 思考模式
- `deepseek-r1` - R1模型
- `deepseek-r1-search` - R1带搜索
- `deepseek-think-search` - 思考模式带搜索
- `deepseek-think-silent` - 静默思考模式
- `deepseek-r1-silent` - 静默R1模式
- `deepseek-search-silent` - 静默搜索模式
- `deepseek-think-fold` - 折叠思考模式
- `deepseek-r1-fold` - 折叠R1模式

## 环境变量

```bash
# 服务配置
PORT=3000
HOST=0.0.0.0

# 日志级别
RUST_LOG=info

# API密钥存储路径
API_KEYS_STORAGE_PATH=./data/api_keys.json

# DeepSeek配置（可选，用于兼容模式）
DEEPSEEK_BASE_URL=https://chat.deepseek.com
DEEPSEEK_AUTHORIZATION=your-fallback-token
```

## Docker部署

```bash
# 构建镜像
docker build -t deepseek-free-api-rust .

# 运行容器
docker run -p 3000:3000 \
  -v $(pwd)/data:/app/data \
  -e RUST_LOG=info \
  deepseek-free-api-rust
```

使用docker-compose：
```bash
docker-compose up -d
```

## 测试

运行测试脚本：
```bash
./test_login.sh
```

该脚本会：
1. 检查服务状态
2. 创建API密钥
3. 测试登录功能
4. 添加账户到API密钥
5. 使用API密钥进行聊天测试

## 工作原理

### 登录流程
1. 客户端提供DeepSeek用户名和密码
2. 系统使用这些凭据访问DeepSeek登录接口
3. 成功登录后，从响应或后续请求中提取userToken
4. 验证userToken的有效性
5. 将userToken存储并关联到API密钥

### 多账户轮换
- 每个API密钥可以关联多个DeepSeek账户
- 请求时随机选择一个可用的userToken
- 自动处理token失效和轮换

### 数据持久化
- API密钥和账户信息存储在JSON文件中
- 支持服务重启后恢复状态
- 定期清理过期的API密钥

## 注意事项

1. **安全性**：请妥善保管API密钥和用户凭据
2. **速率限制**：遵守DeepSeek的API使用限制
3. **Token有效性**：userToken可能会过期，系统会自动处理
4. **网络要求**：需要能够访问DeepSeek官方网站

## 故障排除

### 常见问题

1. **登录失败**
   - 检查用户名密码是否正确
   - 确认DeepSeek账户状态正常
   - 查看日志获取详细错误信息

2. **Token获取失败**
   - DeepSeek可能更新了登录流程
   - 检查网络连接
   - 尝试手动登录验证账户状态

3. **API密钥无效**
   - 检查密钥格式是否正确
   - 确认密钥未过期
   - 验证是否有关联的账户

### 日志查看

```bash
# 查看详细日志
RUST_LOG=debug cargo run

# 或者在Docker中
docker logs deepseek-free-api-rust
```

## 开发

### 项目结构
```
src/
├── main.rs                     # 程序入口
├── config.rs                   # 配置管理
├── error.rs                    # 错误处理
├── models.rs                   # 数据模型
├── utils.rs                    # 工具函数
├── handlers/                   # HTTP处理器
│   ├── mod.rs
│   ├── chat.rs                 # 聊天接口
│   ├── health.rs               # 健康检查
│   ├── token.rs                # Token验证
│   └── api_keys.rs             # API密钥管理
└── services/                   # 业务逻辑
    ├── mod.rs
    ├── deepseek_client.rs      # DeepSeek客户端
    ├── token_manager.rs        # Token管理
    ├── challenge_solver.rs     # 挑战解决
    ├── message_processor.rs    # 消息处理
    ├── login_service.rs        # 登录服务
    └── api_key_manager.rs      # API密钥管理
```

## 贡献

欢迎提交Issue和Pull Request来改进这个项目。

## 许可证

ISC License
