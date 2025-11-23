#!/bin/bash
set -e

# 日志辅助函数
log() {
    echo "[entrypoint] $1"
}

# 默认 UID/GID
PUID=${PUID:-1000}
PGID=${PGID:-1000}

log "启动中，使用 UID=${PUID}, GID=${PGID}"

# 如果需要则调整权限
if [ "$(id -u appuser)" != "$PUID" ] || [ "$(id -g appuser)" != "$PGID" ]; then
    log "正在调整用户/组 ID..."
    
    # 更新组 ID
    if [ "$(id -g appuser)" != "$PGID" ]; then
        groupdel appuser 2>/dev/null || true
        groupadd -g "$PGID" appuser
    fi
    
    # 更新用户 ID
    if [ "$(id -u appuser)" != "$PUID" ]; then
        userdel appuser 2>/dev/null || true
        useradd -u "$PUID" -g appuser -m -s /bin/bash appuser
    fi
    
    # 修复权限
    log "正在更新目录权限..."
    chown -R appuser:appuser /app /config /hooks 2>/dev/null || true
fi

# 验证环境变量
if [ -z "$SUB_URL" ]; then
    echo "[错误] 未设置 SUB_URL！"
    echo "请设置 SUB_URL 环境变量。"
    exit 1
fi

# 检查二进制文件
if [ ! -f "$1" ]; then
    echo "[错误] 未找到二进制文件: $1"
    exit 1
fi

log "正在执行应用程序..."

# 以 appuser 身份执行应用程序
# 使用 'exec' 替换 shell 进程，确保信号正确传递
# Debian 使用 gosu
exec gosu appuser "$@"
