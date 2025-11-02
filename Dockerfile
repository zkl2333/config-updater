# Build stage
FROM rust:1.83-alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /build

# Copy manifests
COPY Cargo.toml ./

# Copy source
COPY src ./src

# Build with release optimizations
RUN cargo build --release

# Runtime stage
FROM alpine:latest

RUN apk add --no-cache ca-certificates bash curl

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/config-updater /app/config-updater

# Create config directory
RUN mkdir -p /config

# Set executable permissions
RUN chmod +x /app/config-updater

ENTRYPOINT ["/app/config-updater"]

