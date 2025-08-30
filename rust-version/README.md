# DeepSeek Free API Server (Rust版本)

## 简介

这是DeepSeek Free API的Rust重写版本，提供与OpenAI兼容的API接口，支持多账号轮询、流式输出、深度思考和联网搜索等功能。

## 功能特性

- ✅ **多账号轮询**: 支持多个userToken轮询使用，提高可用性
- ✅ **OpenAI兼容**: 完全兼容OpenAI Chat Completions API
- ✅ **流式输出**: 支持Server-Sent Events (SSE)流式响应
- ✅ **深度思考**: 支持DeepSeek的R1深度思考模式
- ✅ **联网搜索**: 支持联网搜索功能
- ✅ **挑战应答**: 自动处理DeepSeek的POW挑战机制
- ✅ **高性能**: 基于Rust + Tokio异步运行时
- ✅ **内存安全**: Rust的内存安全保证
- ✅ **并发处理**: 高并发请求处理能力

## 支持的模型

- `deepseek` - 基础对话模型
- `deepseek-search` - 联网搜索模式
- `deepseek-think` / `deepseek-r1` - 深度思考模式
- `deepseek-r1-search` / `deepseek-think-search` - 深度思考+联网搜索
- `deepseek-think-silent` / `deepseek-r1-silent` - 静默思考模式
- `deepseek-search-silent` - 静默搜索模式
- `deepseek-think-fold` / `deepseek-r1-fold` - 折叠思考模式

## 快速开始

### 环境要求

- Rust 1.70+
- SHA3 WASM文件 (用于挑战计算)

### 安装和运行

1. **克隆项目**
```bash
git clone <repository-url>
cd deepseek-free-api/rust-version
```

2. **配置环境变量**
```bash
# 复制环境变量模板
cp .env.example .env

# 编辑配置文件
vim .env
```

3. **编译运行**
```bash
# 开发模式
cargo run

# 生产构建
cargo build --release
./target/release/deepseek-free-api
```

## 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `HOST` | `0.0.0.0` | 服务绑定地址 |
| `PORT` | `8000` | 服务端口 |
| `DEEP_SEEK_CHAT_AUTHORIZATION` | - | 预设的userToken，多个用逗号分隔 |
| `DEEPSEEK_BASE_URL` | `https://chat.deepseek.com` | DeepSeek API地址 |
| `WASM_PATH` | `./sha3_wasm_bg.7b9ca65ddd.wasm` | WASM文件路径 |
| `ENVIRONMENT` | `development` | 运行环境 |

## API接口

### 聊天补全

```bash
POST /v1/chat/completions
Content-Type: application/json
Authorization: Bearer YOUR_USER_TOKEN

{
  "model": "deepseek",
  "messages": [
    {
      "role": "user",
      "content": "你好，介绍一下你自己"
    }
  ],
  "stream": false
}
```

### 模型列表

```bash
GET /v1/models
```

### Token状态检查

```bash
POST /token/check
Content-Type: application/json

{
  "token": "YOUR_USER_TOKEN"
}
```

### 健康检查

```bash
GET /ping
```

## 多账号配置

支持两种方式配置多账号：

1. **环境变量方式**：
```bash
export DEEP_SEEK_CHAT_AUTHORIZATION="token1,token2,token3"
```

2. **请求头方式**：
```bash
Authorization: Bearer token1,token2,token3
```

## Docker部署

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/deepseek-free-api /usr/local/bin/
COPY --from=builder /app/sha3_wasm_bg.7b9ca65ddd.wasm /usr/local/bin/
EXPOSE 8000
CMD ["deepseek-free-api"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  deepseek-api:
    build: .
    ports:
      - "8000:8000"
    environment:
      - DEEP_SEEK_CHAT_AUTHORIZATION=your_tokens_here
      - RUST_LOG=info
    restart: unless-stopped
```

## 性能特性

- **异步处理**: 基于Tokio异步运行时
- **连接池**: HTTP客户端连接复用
- **内存高效**: Rust零拷贝和所有权系统
- **并发安全**: 使用Arc + RwLock实现线程安全
- **错误恢复**: 自动重试和降级处理

## 注意事项

⚠️ **免责声明**

- 本项目仅供学习和研究使用
- 请勿将此项目用于商业用途
- 使用本项目可能存在账号被封禁的风险
- 建议使用官方付费API服务

## 开发

### 项目结构

```
src/
├── main.rs              # 程序入口
├── config.rs            # 配置管理
├── error.rs             # 错误处理
├── models.rs            # 数据模型
├── utils.rs             # 工具函数
├── handlers/            # HTTP处理器
│   ├── mod.rs
│   ├── chat.rs
│   ├── health.rs
│   └── token.rs
└── services/            # 业务逻辑
    ├── mod.rs
    ├── deepseek_client.rs
    ├── token_manager.rs
    ├── challenge_solver.rs
    └── message_processor.rs
```

### 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test token_manager

# 带输出的测试
cargo test -- --nocapture
```

### 日志

使用`RUST_LOG`环境变量控制日志级别：

```bash
# 详细日志
export RUST_LOG=debug

# 生产环境
export RUST_LOG=info

# 特定模块日志
export RUST_LOG=deepseek_free_api=debug,reqwest=info
```

## 贡献

欢迎提交Issue和Pull Request。

## 许可证

ISC License
