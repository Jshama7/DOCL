use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 游戏根目录（含 versions / assets / libraries）
    pub game_dir: String,
    /// 手动指定的 java.exe
    pub java_path: Option<String>,
    /// 最大内存（MB）
    pub memory_mb: u32,
    /// 额外 JVM 参数
    pub java_args: String,
    pub window_width: u32,
    pub window_height: u32,
    pub fullscreen: bool,
    /// auto / official / mirror
    pub download_source: String,
    pub concurrency: u32,
    pub animations: bool,
    pub version_isolation: bool,
    pub selected_version: Option<String>,
    pub selected_account: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_dir: default_game_dir().to_string_lossy().to_string(),
            java_path: None,
            memory_mb: 4096,
            java_args: String::new(),
            window_width: 1280,
            window_height: 720,
            fullscreen: false,
            download_source: "auto".into(),
            concurrency: 8,
            animations: true,
            version_isolation: true,
            selected_version: None,
            selected_account: None,
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("McLauncher")
}

pub fn runtime_dir() -> PathBuf {
    data_dir().join("runtime")
}

pub fn default_game_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn accounts_path() -> PathBuf {
    data_dir().join("accounts.json")
}

pub fn load() -> Config {
    fs::read(config_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(cfg: &Config) -> Result<()> {
    fs::create_dir_all(data_dir())?;
    fs::write(config_path(), serde_json::to_vec_pretty(cfg)?)?;
    Ok(())
}

impl Config {
    pub fn game_root(&self) -> PathBuf {
        PathBuf::from(&self.game_dir)
    }

    pub fn use_mirror(&self) -> bool {
        self.download_source != "official"
    }

    pub fn launch_dir(&self, version_id: &str) -> PathBuf {
        if self.version_isolation {
            self.game_root().join("versions").join(version_id)
        } else {
            self.game_root()
        }
    }
}
