# Hooks 示例

此目录包含 Hook 脚本示例。Hook 脚本会在配置更新时自动执行。

## 快速开始

### 1. 准备 Hook 脚本

**重要说明**：
- 📝 `mihomo.sh` 是提供的**示例脚本**（仅供参考）
- ✅ `post-update` 和 `on-error` 是**实际生效的钩子名称**

**使用方式**：

```bash
# 方式1：复制示例脚本
cp hooks/mihomo.sh hooks/post-update

# 方式2：根据示例编辑自定义脚本
vi hooks/post-update
```

### 2. 设置执行权限

**⚠️ 重要**：必须给脚本添加执行权限

```bash
chmod +x hooks/post-update
```

**验证权限**：
```bash
ls -l hooks/
# 应该显示 -rwxr-xr-x (有 x 执行权限)
```

### 3. 挂载到容器

**推荐方式**：挂载整个 hooks 目录

在 `docker-compose.yaml` 中：

```yaml
config-updater:
  volumes:
    - ./config:/config
    - ./hooks:/hooks:ro
```

这样可以：
- ✅ 同时使用多个钩子（post-update 和 on-error）
- ✅ 避免单文件挂载时可能创建成目录的问题
- ✅ 更容易管理和更新钩子脚本

## Hook 路径说明

容器内有两个固定的 Hook 路径：

| 容器内路径 | 触发时机 | 用途 |
|----------|---------|------|
| `/hooks/post-update` | 配置更新成功后 | 重载服务、发送通知 |
| `/hooks/on-error` | 更新失败时 | 错误通知、告警 |

**推荐做法**：
- 在宿主机 `hooks/` 目录下直接创建 `post-update` 和 `on-error` 脚本
- 挂载整个 `./hooks:/hooks:ro` 目录到容器
- 如果不需要某个钩子，直接不创建对应文件即可

## 示例脚本

### Mihomo 重载

参考 `hooks/mihomo.sh`，或直接使用：

```bash
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
```

**使用**：`cp hooks/mihomo.sh hooks/post-update && chmod +x hooks/post-update`

### Clash 重载

```bash
#!/bin/sh
set -e

# Clash API 地址
CLASH_API="http://clash:9090"

echo "重载 Clash 配置..."

curl -s -X PUT "$CLASH_API/configs?force=true" \
    -H "Content-Type: application/json" \
    -d '{"path": "/root/.config/clash/config.yaml"}'

echo "配置重载成功"
exit 0
```

### 错误通知 (Telegram)

```bash
#!/bin/sh

TELEGRAM_BOT_TOKEN="your_bot_token"
TELEGRAM_CHAT_ID="your_chat_id"
MESSAGE="⚠️ 配置更新失败: $(date '+%Y-%m-%d %H:%M:%S')"

curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
     -d "chat_id=${TELEGRAM_CHAT_ID}" \
     -d "text=${MESSAGE}"

exit 0
```

### 错误通知 (Bark - iOS)

```bash
#!/bin/sh

BARK_URL="https://api.day.app/your_key"

curl -s "${BARK_URL}/配置更新失败/时间:$(date '+%H:%M:%S')?group=config-updater&level=timeSensitive"

exit 0
```

### 通用重启容器

```bash
#!/bin/sh

# 通过 Docker 重启服务（需要挂载 docker.sock）
docker restart mihomo

exit 0
```

**注意**：如果使用 `docker` 命令，需要在 docker-compose.yaml 中挂载 Docker socket：

```yaml
config-updater:
  volumes:
    - /var/run/docker.sock:/var/run/docker.sock:ro
```

## 环境变量

Hook 脚本执行时会设置以下环境变量：

- `CONFIG_PATH`: 配置文件路径（如 `/config/config.yaml`）

**使用示例**：

```bash
#!/bin/sh
echo "配置文件已更新: $CONFIG_PATH"
```

## 调试技巧

### 查看 Hook 执行日志

```bash
docker-compose logs -f config-updater
```

### 成功日志示例

```
[INFO] 正在执行钩子脚本: /hooks/post-update
[INFO] 钩子脚本输出: 重载 Mihomo 配置...
[INFO] 钩子脚本输出: 配置重载成功
[INFO] 钩子脚本执行成功
```

### 失败日志示例

```
[WARN] 权限检查失败: 钩子脚本没有执行权限，请运行: chmod +x /hooks/post-update
[WARN] 提示：在宿主机上运行 'chmod +x /hooks/post-update' 并重启容器
[ERROR] 钩子脚本执行失败: permission denied
```

### 常见问题

**问题 1**: `permission denied`

**解决**：
```bash
chmod +x hooks/post-update hooks/on-error
docker-compose restart config-updater
```

**问题 2**: Hook 没有执行

**检查**：
1. 是否挂载了 hooks 目录（`./hooks:/hooks:ro`）
2. 脚本文件名是否正确（必须是 `post-update` 或 `on-error`）
3. 脚本是否有执行权限（`chmod +x hooks/post-update`）
4. 脚本开头是否有 `#!/bin/sh`

**问题 3**: Hook 执行失败导致配置回滚

**说明**：这是设计行为，确保服务稳定性。如果 Hook 执行失败，配置会自动恢复到上一个版本。

## 最佳实践

1. **总是添加 `set -e`**：脚本出错时立即退出
2. **返回正确的退出码**：成功返回 `0`，失败返回非 `0`
3. **添加日志输出**：使用 `echo` 输出关键步骤，方便调试
4. **测试脚本**：先在本地测试脚本能否正常执行
5. **使用超时**：对于网络请求，建议设置超时避免阻塞

```bash
#!/bin/sh
set -e  # 出错立即退出

echo "开始执行..."

# 使用超时
curl --max-time 10 -s ...

echo "执行成功"
exit 0
```
