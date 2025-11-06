# Config Updater

极简通用配置自动更新服务，Rust 编写。

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
    image: zkl2333/config-updater
    container_name: config-updater
    restart: always
    environment:
      - SUB_URL=https://your-subscription-url?token=xxx
      - UPDATE_INTERVAL=3600
      - USER_AGENT=Clash/1.18.0
      - CONFIG_PATH=/config/config.yaml
    volumes:
      - ./config.yaml:/config/config.yaml:rw
      - ./hooks:/hooks:ro  # 可选：挂载 hooks 目录以启用更新后重载
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

类似 Git Hooks，将脚本挂载到固定路径即可自动执行。程序会在配置更新后自动调用 Hook 脚本。

### Hook 路径

| 路径 | 触发时机 | 用途 |
|------|---------|------|
| `/hooks/post-update` | 配置更新成功后 | 重载服务、发送通知等 |
| `/hooks/on-error` | 更新失败时 | 错误通知、告警等 |

### 配置步骤

#### 1. 准备 Hook 脚本

项目提供了 `hooks/mihomo.sh` 示例脚本，使用时需要复制并重命名：

```bash
# 方式1：复制示例脚本
cp hooks/mihomo.sh hooks/post-update

# 方式2：创建自定义脚本
cat > hooks/post-update << 'EOF'
#!/bin/sh
set -e

# Mihomo API 地址（容器名称）
MIHOMO_API="http://mihomo:9090"
MIHOMO_CONFIG_PATH="/root/.config/mihomo/config.yaml"

echo "重载 Mihomo 配置..."

# 调用 Mihomo API 重载配置
curl -s -X PUT "$MIHOMO_API/configs?force=true" \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"$MIHOMO_CONFIG_PATH\"}"

echo "配置重载成功"
exit 0
EOF
```

**说明**：
- 📝 `hooks/mihomo.sh` 是提供的示例脚本（仅供参考）
- ✅ `hooks/post-update` 是实际生效的钩子（配置更新后执行）
- ✅ `hooks/on-error` 是错误钩子（更新失败时执行）

#### 2. 设置可执行权限

**⚠️ 重要**：Hook 脚本必须有执行权限，否则会执行失败。

```bash
chmod +x hooks/post-update
chmod +x hooks/on-error
```

**验证权限**：

```bash
ls -l hooks/
# 应该显示 -rwxr-xr-x (有 x 执行权限)
```

#### 3. 挂载到容器

**推荐方式**：在 `docker-compose.yaml` 中挂载整个 hooks 目录：

```yaml
config-updater:
  volumes:
    - ./config.yaml:/config/config.yaml:rw
    - ./hooks:/hooks:ro  # 挂载整个 hooks 目录
```

这样可以：
- ✅ 同时使用多个钩子（`post-update` 和 `on-error`）
- ✅ 避免单文件挂载可能创建成目录的问题
- ✅ 更容易管理和更新钩子脚本
- ✅ 不需要的钩子直接不创建文件即可

**注意事项**：

- ✅ 脚本文件名必须是 `post-update` 或 `on-error`（容器会查找这两个固定路径）
- ✅ 使用 `:ro` (只读) 挂载更安全
- ⚠️ 如果 Hook 执行失败，配置会自动回滚到上一个版本
- 💡 完整的 Hook 示例请查看 [`hooks/README.md`](hooks/README.md)

### 更多 Hook 场景

完整示例请查看 [`hooks/README.md`](hooks/README.md)，以下是快速参考：

#### Clash 重载

```bash
#!/bin/sh
curl -X PUT "http://clash:9090/configs?force=true" \
     -H "Content-Type: application/json" \
     -d '{"path": "/root/.config/clash/config.yaml"}'
```

保存为 `hooks/post-update` 并添加执行权限。

#### 通知推送（Telegram）

```bash
#!/bin/sh
TELEGRAM_BOT_TOKEN="your_bot_token"
TELEGRAM_CHAT_ID="your_chat_id"
MESSAGE="✅ 配置已更新: $(date '+%Y-%m-%d %H:%M:%S')"

curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
     -d "chat_id=${TELEGRAM_CHAT_ID}" \
     -d "text=${MESSAGE}"
```

保存为 `hooks/post-update` 并添加执行权限。

### Hook 调试

如果 Hook 没有执行或执行失败，查看容器日志：

```bash
docker-compose logs -f config-updater
```

常见日志输出：

```
# Hook 执行成功
[INFO] 正在执行钩子脚本: /hooks/post-update
[INFO] 钩子脚本输出: 重载 Mihomo 配置...
[INFO] 钩子脚本输出: 配置重载成功
[INFO] 钩子脚本执行成功

# Hook 执行失败（配置会自动回滚）
[ERROR] 更新后钩子脚本执行失败: 钩子脚本执行失败: permission denied
[WARN] 已从备份恢复配置
```

### 权限问题排查

如果看到 `permission denied` 错误：

1. **检查宿主机权限**：
   ```bash
   ls -l hooks/
   # 确保有 x 权限：-rwxr-xr-x
   ```

2. **重新设置权限**：
   ```bash
   chmod +x hooks/post-update hooks/on-error
   ```

3. **重启容器**：
   ```bash
   docker-compose restart config-updater
   ```

更多 Hook 示例和调试技巧请查看 [`hooks/README.md`](hooks/README.md)

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

- 🚀 极小镜像（~18MB）
- ⚡ Rust 高性能
- 🔍 SHA256 差异检测
- 🔄 失败自动回滚
- 🪝 Git 风格 Hook
- 📦 单一二进制
- 🌏 中文日志输出
- 🔧 自定义 User-Agent

## License

MIT
