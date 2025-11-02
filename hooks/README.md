# Hooks

类似 Git Hooks，将脚本放到固定路径即可自动执行。

## Hook 文件路径

- `/hooks/post-update` - 配置更新成功后执行
- `/hooks/on-error` - 更新失败时执行

## 使用方法

1. 创建你的 Hook 脚本（如 `mihomo.sh`）
2. 在脚本内部配置需要的参数
3. 挂载到容器的固定路径：

```yaml
volumes:
  - ./hooks/mihomo.sh:/hooks/post-update:ro
```

就这么简单！

## 示例

### Mihomo

```bash
#!/bin/sh
set -e

# 在脚本内部配置参数
MIHOMO_API="http://mihomo:9090"
MIHOMO_CONFIG_PATH="/root/.config/mihomo/config.yaml"

curl -s -X PUT "$MIHOMO_API/configs?force=true" \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"$MIHOMO_CONFIG_PATH\"}"

exit 0
```

### Clash

类似 Mihomo，修改 API 地址即可。

### 自定义

```bash
#!/bin/sh
# 你的服务重载逻辑
# 配置文件路径可通过环境变量 CONFIG_PATH 获取

your-reload-command

exit 0  # 成功返回 0，失败返回非 0 会触发回滚
```
