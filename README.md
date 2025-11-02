# Config Updater

极简通用配置自动更新服务，Rust 编写，镜像仅 ~35MB。

## 快速开始

### 最简配置

```yaml
services:
  config-updater:
    image: your-registry/config-updater
    environment:
      - SUB_URL=https://your-subscription-url
    volumes:
      - ./config:/config  # 挂载目录，程序自动创建 config.yaml
```

### 完整配置

```yaml
services:
  config-updater:
    image: your-registry/config-updater
    environment:
      - SUB_URL=https://your-subscription-url
      - UPDATE_INTERVAL=3600
      - USER_AGENT=Clash/1.18.0
      - CONFIG_PATH=/config/config.yaml
    volumes:
      - ./config.yaml:/config/config.yaml:rw
```

## 完整示例（Mihomo + MetacubeXD）

参考实际部署配置：

```yaml
services:
  # Web 控制面板
  metacubexd:
    container_name: metacubexd
    image: ghcr.io/metacubex/metacubexd
    restart: always
    ports:
      - '9097:80'

  # Mihomo 代理服务
  mihomo:
    container_name: mihomo
    image: metacubex/mihomo:Alpha
    restart: always
    ports:
      - '7890:7890'  # 代理端口
      - '9090:9090'  # API 端口
    cap_add:
      - ALL
    volumes:
      - ./:/root/.config/mihomo

  # 配置自动更新服务
  config-updater:
    image: your-registry/config-updater
    container_name: mihomo-config-updater
    restart: always
    environment:
      - SUB_URL=https://your-subscription-url?token=xxx
      - UPDATE_INTERVAL=3600
      - USER_AGENT=Clash/1.18.0
      - CONFIG_PATH=/config/config.yaml
    volumes:
      - ./config.yaml:/config/config.yaml:rw
      - ./hooks/mihomo.sh:/hooks/post-update:ro  # 可选：更新后重载
    depends_on:
      - mihomo
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
| `USER_AGENT` | `clash-config-updater/1.0` | 请求 User-Agent |

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

# Mihomo API 地址
MIHOMO_API="http://mihomo:9090"
MIHOMO_CONFIG_PATH="/root/.config/mihomo/config.yaml"

echo "重载 Mihomo 配置..."

# 调用 Mihomo API 重载配置
curl -s -X PUT "$MIHOMO_API/configs?force=true" \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"$MIHOMO_CONFIG_PATH\"}"

echo "配置重载成功"
exit 0
```

挂载到容器（在 docker-compose.yaml 中）：

```yaml
volumes:
  - ./hooks/mihomo.sh:/hooks/post-update:ro
```

### 其他服务

- **Clash**: 与 Mihomo 类似，修改 API 地址
- **V2Ray/Xray**: 通常会自动检测配置变化，无需 Hook
- **自定义**: 创建你自己的 Hook 脚本

## 卷挂载说明

根据使用场景选择合适的挂载方式：

### 场景一：与 Mihomo/Clash 共用配置目录

Mihomo 和 config-updater 共享同一个配置文件：

```yaml
# Mihomo 服务
mihomo:
  volumes:
    - ./:/root/.config/mihomo  # Mihomo 从当前目录读取 config.yaml

# config-updater 服务
config-updater:
  volumes:
    - ./config.yaml:/config/config.yaml:rw  # 更新当前目录的 config.yaml
```

**说明**：config-updater 更新 `./config.yaml`，Mihomo 从 `./config.yaml` 读取

### 场景二：独立配置目录

config-updater 单独管理配置文件：

```yaml
config-updater:
  volumes:
    - ./config:/config  # 程序在 ./config/config.yaml 创建和更新文件
```

**说明**：配置文件在 `./config/config.yaml`

### 场景三：自定义路径

指定具体的配置文件路径：

```yaml
config-updater:
  environment:
    - CONFIG_PATH=/config/my-config.json
  volumes:
    - ./my-app/config.json:/config/my-config.json:rw
```

**说明**：可以自定义配置文件名称和位置

## 日志查看

```bash
# 查看实时日志
docker-compose logs -f config-updater

# 日志示例（中文输出）
[INFO] 配置更新器已启动
[INFO] 订阅地址: https://***
[INFO] 配置路径: /config/config.yaml
[INFO] 更新间隔: 3600 秒
[INFO] User-Agent: Clash/1.18.0
[INFO] ===== 开始更新配置 =====
[INFO] 正在从以下地址下载配置: https://***
[INFO] 已下载 16156 字节
[INFO] 配置文件已更新: /config/config.yaml
[INFO] 更新后钩子脚本执行完成
[INFO] 配置更新成功完成
```

## 构建

```bash
docker build -t config-updater .
```

## 特性

- 🚀 极小镜像（~35MB）
- ⚡ Rust 高性能
- 🔍 SHA256 差异检测
- 🔄 失败自动回滚
- 🪝 Git 风格 Hook
- 📦 单一二进制
- 🌏 中文日志输出
- 🔧 自定义 User-Agent

## License

MIT
