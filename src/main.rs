use anyhow::{Context, Result};
use log::{error, info, warn};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::time;

#[derive(Debug)]
struct Config {
    sub_url: String,
    config_path: String,
    update_interval: u64,
    min_config_size: u64,
    user_agent: String,
}

// 固定的 Hook 路径（类似 Git hooks）
const POST_UPDATE_HOOK: &str = "/hooks/post-update";
const ON_ERROR_HOOK: &str = "/hooks/on-error";

impl Config {
    fn from_env() -> Result<Self> {
        let sub_url = std::env::var("SUB_URL").context("需要设置 SUB_URL 环境变量")?;

        // 验证 SUB_URL 不为空
        if sub_url.trim().is_empty() {
            anyhow::bail!("SUB_URL 不能为空字符串");
        }

        // 验证 URL 格式
        if !sub_url.starts_with("http://") && !sub_url.starts_with("https://") {
            anyhow::bail!("SUB_URL 必须以 http:// 或 https:// 开头");
        }

        let config_path =
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/config/config.yaml".to_string());

        let update_interval = std::env::var("UPDATE_INTERVAL")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .context("UPDATE_INTERVAL 必须是数字")?;

        let min_config_size = std::env::var("MIN_CONFIG_SIZE")
            .unwrap_or_else(|_| "1024".to_string())
            .parse()
            .context("MIN_CONFIG_SIZE 必须是数字")?;

        let user_agent =
            std::env::var("USER_AGENT").unwrap_or_else(|_| "clash-config-updater/1.0".to_string());

        Ok(Config {
            sub_url,
            config_path,
            update_interval,
            min_config_size,
            user_agent,
        })
    }
}

async fn download_config(url: &str, user_agent: &str) -> Result<Vec<u8>> {
    info!("正在从以下地址下载配置: {}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent)
        .build()?;

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP 错误: {}", response.status());
    }

    let bytes = response.bytes().await?;
    info!("已下载 {} 字节", bytes.len());

    Ok(bytes.to_vec())
}

fn calculate_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn is_config_changed(config_path: &str, new_data: &[u8]) -> Result<bool> {
    if !Path::new(config_path).exists() {
        info!("配置文件不存在，将创建新文件");
        return Ok(true);
    }

    let current_data = fs::read(config_path).context("读取当前配置失败")?;

    if current_data.is_empty() {
        info!("当前配置为空");
        return Ok(true);
    }

    let current_hash = calculate_hash(&current_data);
    let new_hash = calculate_hash(new_data);

    if current_hash == new_hash {
        info!("配置未变化 (哈希: {})", current_hash);
        Ok(false)
    } else {
        info!(
            "配置已变化 (旧: {}, 新: {})",
            &current_hash[..8],
            &new_hash[..8]
        );
        Ok(true)
    }
}

fn ensure_config_dir(config_path: &str) -> Result<()> {
    if let Some(parent) = Path::new(config_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).context("创建配置目录失败")?;
            info!("已创建配置目录: {}", parent.display());
        }
    }
    Ok(())
}

fn backup_config(config_path: &str) -> Result<()> {
    if Path::new(config_path).exists() {
        let backup_path = format!("{}.bak", config_path);
        fs::copy(config_path, &backup_path).context("备份配置失败")?;
        info!("已备份配置到: {}", backup_path);
    }
    Ok(())
}

fn restore_backup(config_path: &str) -> Result<()> {
    let backup_path = format!("{}.bak", config_path);
    if Path::new(&backup_path).exists() {
        fs::copy(&backup_path, config_path).context("恢复备份失败")?;
        warn!("已从备份恢复配置");
        Ok(())
    } else {
        anyhow::bail!("未找到备份文件")
    }
}

#[cfg(unix)]
fn check_hook_permissions(hook_path: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(hook_path).context("无法读取钩子脚本元数据")?;

    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // 检查是否有执行权限（所有者、组或其他用户）
    let is_executable = (mode & 0o111) != 0;

    if !is_executable {
        anyhow::bail!("钩子脚本没有执行权限，请运行: chmod +x {}", hook_path);
    }

    Ok(())
}

#[cfg(not(unix))]
fn check_hook_permissions(_hook_path: &str) -> Result<()> {
    // Windows 系统不需要检查执行权限
    Ok(())
}

fn execute_hook(hook_path: &str, config_path: &str) -> Result<()> {
    info!("正在执行钩子脚本: {}", hook_path);

    // 检查权限
    if let Err(e) = check_hook_permissions(hook_path) {
        warn!("权限检查失败: {}", e);
        warn!("提示：在宿主机上运行 'chmod +x {}' 并重启容器", hook_path);
    }

    let output = Command::new(hook_path)
        .env("CONFIG_PATH", config_path)
        .output()
        .context("执行钩子脚本失败")?;

    if output.status.success() {
        info!("钩子脚本执行成功");
        if !output.stdout.is_empty() {
            info!("钩子脚本输出: {}", String::from_utf8_lossy(&output.stdout));
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("钩子脚本执行失败: {}", stderr)
    }
}

async fn update_config(config: &Config) -> Result<()> {
    info!("===== 开始更新配置 =====");

    // Download new config
    let new_data = download_config(&config.sub_url, &config.user_agent)
        .await
        .context("下载配置失败")?;

    // Validate size
    if (new_data.len() as u64) < config.min_config_size {
        anyhow::bail!(
            "下载的配置文件太小: {} 字节 (最小值: {})",
            new_data.len(),
            config.min_config_size
        );
    }

    // Check if changed
    if !is_config_changed(&config.config_path, &new_data)? {
        info!("配置未变化，跳过更新");
        return Ok(());
    }

    // Ensure config directory exists
    ensure_config_dir(&config.config_path)?;

    // Backup current config
    backup_config(&config.config_path)?;

    // Write new config
    fs::write(&config.config_path, &new_data).context("写入新配置失败")?;
    info!("配置文件已更新: {}", config.config_path);

    // Execute post-update hook
    if Path::new(POST_UPDATE_HOOK).exists() {
        match execute_hook(POST_UPDATE_HOOK, &config.config_path) {
            Ok(_) => info!("更新后钩子脚本执行完成"),
            Err(e) => {
                error!("更新后钩子脚本执行失败: {}", e);
                // Restore backup on hook failure
                if let Err(restore_err) = restore_backup(&config.config_path) {
                    error!("恢复备份失败: {}", restore_err);
                }
                return Err(e);
            }
        }
    }

    info!("配置更新成功完成");
    Ok(())
}

async fn run_updater(config: Config) {
    info!("配置更新器已启动");
    info!("订阅地址: {}", config.sub_url);
    info!("配置路径: {}", config.config_path);
    info!("更新间隔: {} 秒", config.update_interval);
    info!("User-Agent: {}", config.user_agent);

    let interval = Duration::from_secs(config.update_interval);
    let mut interval_timer = time::interval(interval);

    // 立即执行第一次更新（tick 的第一次调用会立即返回）
    info!("准备执行首次配置更新...");

    let mut iteration = 0u64;
    loop {
        iteration += 1;
        info!("===== 第 {} 次更新循环 =====", iteration);

        interval_timer.tick().await;
        info!("定时器触发，开始更新...");

        match update_config(&config).await {
            Ok(_) => {
                info!("✓ 第 {} 次更新周期完成", iteration);
            }
            Err(e) => {
                error!("✗ 第 {} 次更新失败: {}", iteration, e);
                error!("错误详情: {:?}", e);

                // Execute error hook if exists
                if Path::new(ON_ERROR_HOOK).exists() {
                    if let Err(hook_err) = execute_hook(ON_ERROR_HOOK, &config.config_path) {
                        error!("错误钩子脚本执行失败: {}", hook_err);
                    }
                }
            }
        }

        info!("等待 {} 秒后进行下次更新...", config.update_interval);
        info!(
            "下次更新时间: {:?}",
            std::time::SystemTime::now() + interval
        );
    }
}

#[tokio::main]
async fn main() {
    // 设置 panic hook 来捕获所有 panic
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("!!! 程序发生 PANIC !!!");
        eprintln!("Panic 信息: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!(
                "位置: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        eprintln!("请将此信息报告给开发者");
        // 确保日志被刷新
        std::io::Write::flush(&mut std::io::stderr()).ok();
    }));

    // 在日志系统初始化之前先输出到 stderr，确保能看到启动信息
    eprintln!("=== Config Updater 启动 ===");
    eprintln!("进程 PID: {}", std::process::id());

    // 初始化日志系统，确保错误信息能够输出
    // 使用 write_style Always 确保在容器中也能正常显示
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .format_target(false)
        .write_style(env_logger::WriteStyle::Always)
        .init();

    info!("程序启动中...");
    info!("版本: {}", env!("CARGO_PKG_VERSION"));
    info!("进程 PID: {}", std::process::id());

    // 输出环境变量信息（不包含敏感信息）
    info!("环境变量检查:");
    let sub_url_status = match std::env::var("SUB_URL") {
        Ok(val) => {
            if val.is_empty() {
                "已设置但为空".to_string()
            } else {
                format!("已设置 (长度: {} 字符)", val.len())
            }
        }
        Err(_) => "未设置".to_string(),
    };
    info!("  SUB_URL: {}", sub_url_status);
    info!(
        "  CONFIG_PATH: {}",
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "使用默认值".to_string())
    );
    info!(
        "  UPDATE_INTERVAL: {}",
        std::env::var("UPDATE_INTERVAL").unwrap_or_else(|_| "使用默认值 3600".to_string())
    );

    // 加载配置，如果失败则输出详细错误信息
    let config = match Config::from_env() {
        Ok(cfg) => {
            info!("✓ 配置加载成功");
            cfg
        }
        Err(e) => {
            error!("配置加载失败: {}", e);
            eprintln!("错误: {}", e);
            eprintln!("请确保设置了必需的环境变量 SUB_URL");
            std::process::exit(1);
        }
    };

    info!("开始运行配置更新器");
    run_updater(config).await;

    // 理论上不应该到达这里，因为 run_updater 是无限循环
    error!("!!! 配置更新器意外退出 !!!");
    eprintln!("!!! 配置更新器意外退出 !!!");
    std::process::exit(1);
}
