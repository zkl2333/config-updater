#!/bin/sh
set -e

# 默认 UID 和 GID
PUID=${PUID:-1000}
PGID=${PGID:-1000}

echo "Starting with PUID=${PUID} and PGID=${PGID}"

# 如果当前用户 ID 和 GID 与期望的不同，则调整
if [ "$(id -u appuser)" != "$PUID" ] || [ "$(id -g appuser)" != "$PGID" ]; then
    echo "Adjusting user and group IDs..."
    
    # 修改组 ID
    if [ "$(id -g appuser)" != "$PGID" ]; then
        delgroup appuser 2>/dev/null || true
        addgroup -g "$PGID" appuser 2>/dev/null || true
    fi
    
    # 修改用户 ID
    if [ "$(id -u appuser)" != "$PUID" ]; then
        deluser appuser 2>/dev/null || true
        adduser -D -u "$PUID" -G appuser appuser 2>/dev/null || true
    fi
    
    # 更新目录权限
    echo "Updating directory permissions..."
    chown -R appuser:appuser /app /config /hooks 2>/dev/null || true
fi

# 切换到 appuser 用户并执行应用
echo "Starting application as user appuser (${PUID}:${PGID})"
exec su-exec appuser "$@"

