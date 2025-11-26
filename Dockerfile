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
    PGID=1000

ENTRYPOINT ["/entrypoint.sh"]

CMD ["/app/config-updater"]
