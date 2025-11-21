#!/bin/sh
set -e

echo "========================================"
echo "=== Entrypoint Script Started ==="
echo "========================================"
echo "Date: $(date '+%Y-%m-%d %H:%M:%S')"
echo "Script: $0"
echo "Args: $@"
echo ""

# 默认 UID 和 GID
PUID=${PUID:-1000}
PGID=${PGID:-1000}

echo ">>> Configuration:"
echo "    PUID=${PUID}"
echo "    PGID=${PGID}"
echo "    SUB_URL=${SUB_URL:-<NOT SET>}"
echo "    CONFIG_PATH=${CONFIG_PATH:-/config/config.yaml}"
echo "    UPDATE_INTERVAL=${UPDATE_INTERVAL:-3600}"
echo ""

# 检查二进制文件
echo ">>> Checking binary file:"
if [ -f "$1" ]; then
    echo "    File: $1"
    echo "    Exists: YES"
    echo "    Size: $(stat -c%s "$1" 2>/dev/null || stat -f%z "$1" 2>/dev/null || echo 'unknown') bytes"
    echo "    Permissions: $(ls -l "$1")"
else
    echo "    ERROR: Binary file not found: $1"
    exit 1
fi
echo ""

# 如果当前用户 ID 和 GID 与期望的不同，则调整
if [ "$(id -u appuser)" != "$PUID" ] || [ "$(id -g appuser)" != "$PGID" ]; then
    echo ">>> Adjusting user and group IDs..."
    
    # 修改组 ID
    if [ "$(id -g appuser)" != "$PGID" ]; then
        echo "    Updating group to $PGID..."
        delgroup appuser 2>/dev/null || true
        addgroup -g "$PGID" appuser 2>/dev/null || true
    fi
    
    # 修改用户 ID
    if [ "$(id -u appuser)" != "$PUID" ]; then
        echo "    Updating user to $PUID..."
        deluser appuser 2>/dev/null || true
        adduser -D -u "$PUID" -G appuser appuser 2>/dev/null || true
    fi
    
    # 更新目录权限
    echo "    Updating directory permissions..."
    chown -R appuser:appuser /app /config /hooks 2>/dev/null || true
    echo "    Done."
else
    echo ">>> User IDs already correct, skipping adjustment."
fi
echo ""

# 验证必需环境变量
echo ">>> Validating environment:"
if [ -z "$SUB_URL" ]; then
    echo "    ERROR: SUB_URL is not set!"
    echo "    Please set SUB_URL environment variable."
    echo "    Example: -e SUB_URL=https://your-subscription-url"
    exit 1
fi
echo "    SUB_URL: OK"
echo ""

# 切换到 appuser 用户并执行应用
echo ">>> Starting application as user appuser (${PUID}:${PGID})"
echo "    Command: $@"
echo "========================================"
echo ""

# 不使用 exec，这样可以捕获退出码
su-exec appuser "$@"
EXIT_CODE=$?

echo ""
echo "========================================"
echo "!!! Application exited with code: $EXIT_CODE"
echo "Date: $(date '+%Y-%m-%d %H:%M:%S')"
echo "========================================"

exit $EXIT_CODE

