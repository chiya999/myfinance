mod settings;

pub use settings::Settings;

use crate::error::AppResult;
use std::path::PathBuf;

/// 配置文件路径
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("myfinance")
        .join("config.toml")
}

/// 加载配置，不存在则返回默认值
pub fn load_config() -> AppResult<Settings> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let settings: Settings = toml::from_str(&content)
            .map_err(|e| crate::error::AppError::Internal(format!("配置解析错误: {e}")))?;
        Ok(settings)
    } else {
        Ok(Settings::default())
    }
}

/// 保存配置
pub fn save_config(settings: &Settings) -> AppResult<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(settings)
        .map_err(|e| crate::error::AppError::Internal(format!("配置序列化错误: {e}")))?;
    std::fs::write(&path, content)?;
    Ok(())
}
