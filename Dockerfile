# Build stage
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /build

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy real source code
COPY src ./src

# Touch main.rs to force rebuild of our code only
RUN touch src/main.rs && \
    cargo build --release && \
    strip target/release/config-updater

# Runtime stage
FROM alpine:3.19

# Install only essential runtime dependencies
RUN apk add --no-cache ca-certificates && \
    rm -rf /var/cache/apk/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/config-updater /app/config-updater

# Create config and hooks directories
RUN mkdir -p /config /hooks

ENTRYPOINT ["/app/config-updater"]

