FROM alpine:3.21

RUN apk add --no-cache ca-certificates curl wget su-exec bash && \
    rm -rf /var/cache/apk/*

RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

WORKDIR /app

# 根据 buildx 的 TARGETARCH 动态复制正确的二进制
ARG TARGETARCH
COPY ./dist/config-updater-${TARGETARCH} /app/config-updater
RUN chmod +x /app/config-updater

COPY entrypoint.sh /entrypoint.sh
RUN sed -i 's/\r$//' /entrypoint.sh && \
    chmod +x /entrypoint.sh

RUN mkdir -p /config /hooks && \
    chown -R appuser:appuser /app /config /hooks

ENV PUID=1000 \
    PGID=1000 \
    BUILD_TIME=""

# 设置构建时间（在构建时会被覆盖）
ARG BUILD_TIME_ARG
RUN if [ -n "$BUILD_TIME_ARG" ]; then \
        echo "export BUILD_TIME=\"$BUILD_TIME_ARG\"" >> /etc/profile.d/build-time.sh; \
    else \
        echo "export BUILD_TIME=\"$(date -u +'%Y-%m-%d %H:%M:%S UTC')\"" >> /etc/profile.d/build-time.sh; \
    fi

ENTRYPOINT ["/entrypoint.sh"]

CMD ["/app/config-updater"]
