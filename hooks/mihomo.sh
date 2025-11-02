#!/bin/sh
# Mihomo 配置重载 Hook
# 配置更新后自动调用 Mihomo API 重新加载配置

set -e

# === 在这里配置你的参数 ===
MIHOMO_API="http://mihomo:9090"
MIHOMO_CONFIG_PATH="/root/.config/mihomo/config.yaml"
# ========================

echo "[Hook] Reloading Mihomo config..."

# 调用 Mihomo API 重载配置
response=$(curl -s -w "%{http_code}" -X PUT "$MIHOMO_API/configs?force=true" \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"$MIHOMO_CONFIG_PATH\"}")

http_code=${response: -3}

if [ "$http_code" = "204" ]; then
    echo "[Hook] Mihomo config reloaded successfully"
    exit 0
else
    echo "[Hook] Failed to reload Mihomo config (HTTP $http_code)"
    exit 1
fi
