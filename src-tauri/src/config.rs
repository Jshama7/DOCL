use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameDirEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerEntry {
    pub address: String,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 当前使用的游戏根目录（含 versions / assets / libraries）
    pub game_dir: String,
    /// 保存过的游戏目录，可在设置里切换
    pub game_dirs: Vec<GameDirEntry>,
    /// 最近连接过的服务器
    pub recent_servers: Vec<ServerEntry>,
    /// 房间大厅数据地址（返回 JSON 列表），留空表示不启用大厅
    pub hall_url: String,
    /// 自更新清单地址，留空表示不检查更新
    pub update_url: String,
    /// 启动器更新策略：auto（自动下载）/ notify（提示）/ off（关闭）
    pub update_mode: String,
    /// 手动指定的 java.exe
    pub java_path: Option<String>,
    /// 最大内存（MB）
    pub memory_mb: u32,
    /// 自动按系统内存推荐分配
    pub memory_auto: bool,
    /// auto / g1gc / zgc / custom
    pub memory_mode: String,
    /// 额外 JVM 参数
    pub java_args: String,
    /// 额外游戏参数
    pub game_args: String,
    /// 启动前执行的命令
    pub pre_launch: String,
    /// 不再向游戏注入 DOCL 启动器标识
    pub disable_launch_wrapper: bool,
    /// 不再注入 Java 24+ 的 LWJGL Unsafe 兼容参数
    pub disable_lwjgl_unsafe: bool,
    /// 为所选 Java 写入高性能显卡偏好
    pub prefer_gpu: bool,
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
    /// paper / celadon / terracotta / moon / nocturne / sakura / ocean / lavender / graphite
    pub theme: String,
    pub background_image: Option<String>,
    pub background_blur: u32,
    pub glass_opacity: u32,
    pub glass_blur: u32,
    /// 背景遮罩暗度（0-60）
    pub background_dim: u32,
    /// default / sharp / round
    pub ui_radius: String,
    /// compact / normal / comfy
    pub ui_density: String,
    /// Windows Acrylic：窗口毛玻璃化，透出桌面实时画面
    pub window_vibrancy: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_dir: default_game_dir().to_string_lossy().to_string(),
            game_dirs: Vec::new(),
            recent_servers: Vec::new(),
            hall_url: String::new(),
            // 默认走国内加速；玩家可在设置里切到 GitHub 原地址
            update_url: "https://cdn.jsdelivr.net/gh/dawo4/DOCL@main/update.json".into(),
            update_mode: "notify".into(),
            java_path: None,
            memory_mb: 4096,
            memory_auto: true,
            memory_mode: "auto".into(),
            java_args: String::new(),
            game_args: String::new(),
            pre_launch: String::new(),
            disable_launch_wrapper: false,
            disable_lwjgl_unsafe: false,
            prefer_gpu: false,
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
            background_dim: 0,
            ui_radius: "default".into(),
            ui_density: "normal".into(),
            // 便携版默认透出桌面：配合外带场景的玻璃质感；安装版保持实心
            window_vibrancy: is_portable(),
        }
    }
}

/// 便携版标记：exe 同目录存在 portable.txt 或 data 目录时启用。
/// 只读位置（Program Files、只读介质）无法写入时自动回退到用户目录，避免启动即报错。
fn portable_root() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let data = dir.join("data");
    let marked = dir.join("portable.txt").is_file() || data.is_dir();
    if !marked {
        return None;
    }
    fs::create_dir_all(&data).ok()?;
    Some(dir)
}

fn portable_base() -> Option<&'static PathBuf> {
    static CACHE: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    CACHE.get_or_init(portable_root).as_ref()
}

/// 当前是否为便携版（配置、账号、实例索引都放在 exe 同目录的 data 下）。
pub fn is_portable() -> bool {
    portable_base().is_some()
}

pub fn data_dir() -> PathBuf {
    if let Some(root) = portable_base() {
        return root.join("data");
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("McLauncher")
}

pub fn runtime_dir() -> PathBuf {
    data_dir().join("runtime")
}

pub fn default_game_dir() -> PathBuf {
    if let Some(root) = portable_base() {
        return root.join("data").join(".minecraft");
    }
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
