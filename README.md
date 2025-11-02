# Config Updater

极简通用配置自动更新服务，Rust 编写，镜像仅 ~10MB。

## 快速开始

### 最简配置

```yaml
# docker-compose.yaml
services:
  config-updater:
    image: your-registry/config-updater
    environment:
      - SUB_URL=https://your-subscription-url
    volumes:
      - ./config:/config  # 挂载目录，程序自动创建 config.yaml
```

配置文件会自动创建在 `./config/config.yaml`。

**自定义文件名：**
```yaml
environment:
  - SUB_URL=https://your-subscription-url
  - CONFIG_PATH=/config/my-config.yaml  # 自定义文件名
volumes:
  - ./config:/config
```

### 完整示例（Mihomo + Hook）

```yaml
services:
  mihomo:
    image: metacubex/mihomo:Alpha
    volumes:
      - ./config:/root/.config/mihomo

  config-updater:
    image: your-registry/config-updater
    environment:
      - SUB_URL=https://your-subscription-url
      - CONFIG_PATH=/config/config.yaml
    volumes:
      - ./config:/config
      - ./hooks/mihomo.sh:/hooks/post-update:ro
```

## 配置说明

### 必需参数

| 变量 | 说明 |
|------|------|
| `SUB_URL` | 订阅地址 |

### 可选参数

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `CONFIG_PATH` | `/config/config.yaml` | 配置文件路径 |
| `UPDATE_INTERVAL` | `3600` | 更新间隔（秒） |
| `MIN_CONFIG_SIZE` | `1024` | 最小配置大小（字节） |

## Hook 机制（可选）

类似 Git Hooks，将脚本挂载到固定路径即可自动执行。

### Hook 路径

- `/hooks/post-update` - 配置更新成功后执行
- `/hooks/on-error` - 更新失败时执行

### Mihomo Hook 示例

创建 `hooks/mihomo.sh`：

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

挂载到容器：

```yaml
volumes:
  - ./hooks/mihomo.sh:/hooks/post-update:ro
```

### 其他服务

- **Clash**: 与 Mihomo 类似，修改 API 地址
- **V2Ray/Xray**: 通常会自动检测配置变化，无需 Hook
- **自定义**: 创建你自己的 Hook 脚本

## 构建

```bash
docker build -t config-updater .
```

## 特性

- 🚀 极小镜像（~10MB）
- ⚡ Rust 高性能
- 🔍 SHA256 差异检测
- 🔄 失败自动回滚
- 🪝 Git 风格 Hook
- 📦 单一二进制
- 🎯 零配置运行

## License

MIT
