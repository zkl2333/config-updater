# Build stage
FROM rust:1.83-slim-bullseye AS builder

WORKDIR /build

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
# 使用缓存挂载来加速依赖下载和构建（包括 target 目录）
# 使用 sharing=private 避免多平台构建时的锁竞争
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target,sharing=private \
    mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy real source code
COPY src ./src

# 使用缓存挂载来加速最终构建
# 编译完成后将二进制文件复制到非缓存位置
# 使用 sharing=private 避免多平台构建时的锁竞争
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target,sharing=private \
    cargo build --release && \
    cp /build/target/release/config-updater /config-updater && \
    strip /config-updater

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies including tools for hooks and gosu for user switching
# ca-certificates, curl, wget 常用工具
# gosu 用于替代 su-exec
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl wget gosu && \
    rm -rf /var/lib/apt/lists/*

# Create default non-root user (UID/GID can be changed at runtime)
# Debian 使用 useradd/groupadd
RUN groupadd -g 1000 appuser && \
    useradd -u 1000 -g appuser -m -s /bin/bash appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /config-updater /app/config-updater

# Copy entrypoint script
COPY entrypoint.sh /entrypoint.sh
# Convert Windows line endings (CRLF) to Unix (LF) and set executable permission
RUN sed -i 's/\r$//' /entrypoint.sh && \
    chmod +x /entrypoint.sh

# Create config and hooks directories with proper permissions
RUN mkdir -p /config /hooks && \
    chown -R appuser:appuser /app /config /hooks

# 环境变量说明（默认值）
ENV PUID=1000 \
    PGID=1000

# 使用 entrypoint 脚本来处理动态 UID/GID
ENTRYPOINT ["/entrypoint.sh"]

# 默认命令
CMD ["/app/config-updater"]
