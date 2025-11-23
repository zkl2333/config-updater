# Build stage
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# 仅拉取依赖，避免占位 main 污染最终二进制
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo fetch --locked && \
    rm -rf src

# Copy real source code
COPY src ./src

# 使用缓存挂载来加速最终构建
# 编译完成后将二进制文件复制到非缓存位置
# 使用 sharing=private 避免多平台构建时的锁竞争
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target,sharing=private \
    cargo build --release --locked && \
    cp /build/target/release/config-updater /config-updater

# Runtime stage
FROM alpine:3.21

# Install runtime dependencies including tools for hooks and su-exec for user switching
RUN apk add --no-cache ca-certificates curl wget su-exec bash && \
    rm -rf /var/cache/apk/*

# Create default non-root user (UID/GID can be changed at runtime)
RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

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
