use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameDirEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 当前使用的游戏根目录（含 versions / assets / libraries）
    pub game_dir: String,
    /// 保存过的游戏目录，可在设置里切换
    pub game_dirs: Vec<GameDirEntry>,
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
    /// paper / celadon / terracotta / moon / nocturne
    pub theme: String,
    pub background_image: Option<String>,
    pub background_blur: u32,
    pub glass_opacity: u32,
    pub glass_blur: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_dir: default_game_dir().to_string_lossy().to_string(),
            game_dirs: Vec::new(),
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
            theme: "paper".into(),
            background_image: Some("builtin:forest".into()),
            background_blur: 4,
            glass_opacity: 62,
            glass_blur: 18,
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

fn dir_label(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);
    std::path::Path::new(trimmed)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn same_path(a: &str, b: &str) -> bool {
    a.trim_end_matches(['\\', '/']).eq_ignore_ascii_case(b.trim_end_matches(['\\', '/']))
}

pub fn accounts_path() -> PathBuf {
    data_dir().join("accounts.json")
}

pub fn load() -> Config {
    let mut config: Config = fs::read(config_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    config.sync_game_dirs();
    config
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

    /// 确保目录列表里包含当前目录，并给缺失名称的条目补上默认名字。
    pub fn sync_game_dirs(&mut self) {
        self.game_dirs.retain(|entry| !entry.path.trim().is_empty());
        for entry in self.game_dirs.iter_mut() {
            if entry.name.trim().is_empty() {
                entry.name = dir_label(&entry.path);
            }
        }
        if !self
            .game_dirs
            .iter()
            .any(|entry| same_path(&entry.path, &self.game_dir))
        {
            self.game_dirs.insert(
                0,
                GameDirEntry {
                    name: dir_label(&self.game_dir),
                    path: self.game_dir.clone(),
                },
            );
        }
    }

    pub fn set_active_dir(&mut self, path: &str) {
        self.game_dir = path.to_string();
        self.sync_game_dirs();
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
