FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

# 安装必要的运行时依赖
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# 复制二进制文件和WASM文件
COPY --from=builder /app/target/release/deepseek-free-api /usr/local/bin/
COPY --from=builder /app/sha3_wasm_bg.7b9ca65ddd.wasm /usr/local/bin/

# 设置工作目录
WORKDIR /usr/local/bin

# 暴露端口
EXPOSE 8000

# 设置环境变量
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8000

# 运行应用
CMD ["deepseek-free-api"]
