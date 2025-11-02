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
}

// 固定的 Hook 路径（类似 Git hooks）
const POST_UPDATE_HOOK: &str = "/hooks/post-update";
const ON_ERROR_HOOK: &str = "/hooks/on-error";

impl Config {
    fn from_env() -> Result<Self> {
        let sub_url = std::env::var("SUB_URL")
            .context("需要设置 SUB_URL 环境变量")?;

        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "/config/config.yaml".to_string());

        let update_interval = std::env::var("UPDATE_INTERVAL")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .context("UPDATE_INTERVAL 必须是数字")?;

        let min_config_size = std::env::var("MIN_CONFIG_SIZE")
            .unwrap_or_else(|_| "1024".to_string())
            .parse()
            .context("MIN_CONFIG_SIZE 必须是数字")?;

        Ok(Config {
            sub_url,
            config_path,
            update_interval,
            min_config_size,
        })
    }
}

async fn download_config(url: &str) -> Result<Vec<u8>> {
    info!("正在从以下地址下载配置: {}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
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

    let current_data = fs::read(config_path)
        .context("读取当前配置失败")?;

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
        info!("配置已变化 (旧: {}, 新: {})", &current_hash[..8], &new_hash[..8]);
        Ok(true)
    }
}

fn ensure_config_dir(config_path: &str) -> Result<()> {
    if let Some(parent) = Path::new(config_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .context("创建配置目录失败")?;
            info!("已创建配置目录: {}", parent.display());
        }
    }
    Ok(())
}

fn backup_config(config_path: &str) -> Result<()> {
    if Path::new(config_path).exists() {
        let backup_path = format!("{}.bak", config_path);
        fs::copy(config_path, &backup_path)
            .context("备份配置失败")?;
        info!("已备份配置到: {}", backup_path);
    }
    Ok(())
}

fn restore_backup(config_path: &str) -> Result<()> {
    let backup_path = format!("{}.bak", config_path);
    if Path::new(&backup_path).exists() {
        fs::copy(&backup_path, config_path)
            .context("恢复备份失败")?;
        warn!("已从备份恢复配置");
        Ok(())
    } else {
        anyhow::bail!("未找到备份文件")
    }
}

fn execute_hook(hook_path: &str, config_path: &str) -> Result<()> {
    info!("正在执行钩子脚本: {}", hook_path);

    let output = Command::new("sh")
        .arg(hook_path)
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
    let new_data = download_config(&config.sub_url).await
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
    fs::write(&config.config_path, &new_data)
        .context("写入新配置失败")?;
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

    let interval = Duration::from_secs(config.update_interval);
    let mut interval_timer = time::interval(interval);

    loop {
        interval_timer.tick().await;

        match update_config(&config).await {
            Ok(_) => info!("更新周期完成"),
            Err(e) => {
                error!("更新失败: {}", e);

                // Execute error hook if exists
                if Path::new(ON_ERROR_HOOK).exists() {
                    if let Err(hook_err) = execute_hook(ON_ERROR_HOOK, &config.config_path) {
                        error!("错误钩子脚本执行失败: {}", hook_err);
                    }
                }
            }
        }

        info!("等待 {} 秒后进行下次更新", config.update_interval);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    let config = Config::from_env()
        .context("加载配置失败")?;

    run_updater(config).await;

    Ok(())
}
