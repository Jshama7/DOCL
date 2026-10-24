use crate::config::Config;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

/// 从 PCL 读到的可迁移项；未找到的字段为 None，不会覆盖现有设置。
#[derive(Debug, Default, Serialize)]
pub struct PclPreview {
    pub root: String,
    pub game_dir: Option<String>,
    pub java_path: Option<String>,
    pub memory_auto: Option<bool>,
    pub memory_mb: Option<u32>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub update_mode: Option<String>,
    pub download_source: Option<String>,
    /// PCL 上次启动的版本（对应 .minecraft\versions 下的目录名）
    pub selected_version: Option<String>,
    pub findings: Vec<String>,
}

fn normalize_root(raw: &str) -> Result<PathBuf> {
    let path = PathBuf::from(raw.trim().trim_matches('"'));
    if !path.exists() {
        bail!("路径不存在：{}", path.display());
    }
    if path.is_file() {
        return path
            .parent()
            .map(Path::to_path_buf)
            .context("无法定位 PCL 所在目录");
    }
    Ok(path)
}

/// PCL 的 Setup.ini 是 `键:值`，个别版本写成 `键=值`，两种都接受。
fn parse_ini(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(index) = line.find([':', '=']) {
            let key = line[..index].trim();
            let value = line[index + 1..].trim();
            if !key.is_empty() {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    map
}

fn reg_value(key: &RegKey, name: &str) -> Option<String> {
    if let Ok(value) = key.get_value::<String, _>(name) {
        return Some(value);
    }
    if let Ok(value) = key.get_value::<u32, _>(name) {
        return Some(value.to_string());
    }
    if let Ok(value) = key.get_value::<u64, _>(name) {
        return Some(value.to_string());
    }
    None
}

const WANTED_KEYS: [&str; 9] = [
    "LaunchRamType",
    "LaunchRamCustom",
    "SetupJavaPath",
    "SystemSystemUpdate",
    "ToolDownloadSource",
    "LaunchFolderSelect",
    "LaunchArgumentWindowWidth",
    "LaunchArgumentWindowHeight",
    "LaunchArgumentIndieV2",
];

/// 注册表是 PCL 的主存储，便携版 Setup.ini 里的同名项优先。
fn read_settings(root: &Path) -> HashMap<String, String> {
    let mut settings = HashMap::new();
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\PCL", KEY_READ) {
        for name in WANTED_KEYS {
            if let Some(value) = reg_value(&key, name) {
                settings.insert(name.to_string(), value);
            }
        }
    }
    for candidate in [root.join("PCL").join("Setup.ini"), root.join("Setup.ini")] {
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            for (key, value) in parse_ini(&text) {
                settings.insert(key, value);
            }
        }
    }
    settings
}

/// `$.minecraft\` 里的 `$` 表示 PCL 目录。
fn resolve_game_dir(root: &Path, settings: &HashMap<String, String>) -> Option<PathBuf> {
    if let Some(raw) = settings.get("LaunchFolderSelect") {
        let raw = raw.trim().trim_matches('"');
        if !raw.is_empty() {
            let path = match raw.strip_prefix('$') {
                Some(rest) => root.join(rest.trim_start_matches(['\\', '/'])),
                None => PathBuf::from(raw),
            };
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    let fallback = root.join(".minecraft");
    fallback.is_dir().then_some(fallback)
}

/// PCL 的 Java 设置一般指向 bin 目录，这里统一换算成 java.exe。
fn resolve_java(root: &Path, settings: &HashMap<String, String>) -> Option<PathBuf> {
    let raw = settings.get("SetupJavaPath")?.trim().trim_matches('"');
    if raw.is_empty() {
        return None;
    }
    let mut path = PathBuf::from(raw);
    if !path.is_absolute() {
        path = root.join(path);
    }
    if path.is_dir() {
        path = path.join("java.exe");
    }
    path.is_file().then_some(path)
}

/// 内存滑条换算：0-12 步 0.1GB，13-25 步 0.5GB，26-33 步 1GB，之后再 2GB 一档。
fn ram_from_slider(value: f64) -> f64 {
    if value <= 12.0 {
        value * 0.1 + 0.3
    } else if value <= 25.0 {
        (value - 12.0) * 0.5 + 1.5
    } else if value <= 33.0 {
        (value - 25.0) + 8.0
    } else {
        (value - 33.0) * 2.0 + 16.0
    }
}

fn read_memory(settings: &HashMap<String, String>) -> (Option<bool>, Option<u32>) {
    match settings.get("LaunchRamType").and_then(|v| v.parse::<i32>().ok()) {
        Some(0) => (Some(true), None),
        Some(1) => {
            let slider = settings
                .get("LaunchRamCustom")
                .and_then(|v| v.parse::<f64>().ok());
            match slider {
                Some(value) => {
                    let mb = ((ram_from_slider(value) * 1024.0).round() as u32).clamp(1024, 32768);
                    (Some(false), Some(mb))
                }
                None => (None, None),
            }
        }
        _ => (None, None),
    }
}

fn read_window(settings: &HashMap<String, String>) -> (Option<u32>, Option<u32>) {
    let parse = |key: &str| settings.get(key).and_then(|v| v.trim().parse::<u32>().ok());
    let width = parse("LaunchArgumentWindowWidth");
    let height = parse("LaunchArgumentWindowHeight");
    if width.is_some() && height.is_some() && width != Some(0) && height != Some(0) {
        (width, height)
    } else {
        (None, None)
    }
}

fn read_update_mode(settings: &HashMap<String, String>) -> Option<String> {
    let mode = match settings.get("SystemSystemUpdate")?.trim().parse::<u32>().ok()? {
        0 => "auto",
        1 | 2 => "notify",
        3 => "off",
        _ => return None,
    };
    Some(mode.to_string())
}

fn read_download_source(settings: &HashMap<String, String>) -> Option<String> {
    let source = match settings.get("ToolDownloadSource")?.trim().parse::<u32>().ok()? {
        0 => "mirror",
        1 => "auto",
        2 => "official",
        _ => return None,
    };
    Some(source.to_string())
}

/// PCL 把上次启动的版本记在 .minecraft\PCL.ini 的 Version 项里。
fn read_selected_version(game_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(game_dir.join("PCL.ini")).ok()?;
    let version = parse_ini(&text).remove("Version")?;
    let version = version.trim().to_string();
    if version.is_empty() || !game_dir.join("versions").join(&version).is_dir() {
        return None;
    }
    Some(version)
}

fn update_mode_label(mode: &str) -> &'static str {
    match mode {
        "auto" => "有新版本时自动下载",
        "notify" => "有新版本时提示",
        _ => "关闭自动检查",
    }
}

fn download_source_label(source: &str) -> &'static str {
    match source {
        "mirror" => "优先镜像源",
        "official" => "优先官方源",
        _ => "官方源优先，慢时切镜像",
    }
}

pub fn inspect(raw_root: &str) -> Result<PclPreview> {
    let root = normalize_root(raw_root)?;
    let looks_like_pcl = root.join("PCL").is_dir()
        || root.join(".minecraft").is_dir()
        || root.join("Setup.ini").is_file();
    if !looks_like_pcl {
        bail!("这个文件夹不像 PCL 目录：没有找到 PCL 或 .minecraft 子目录");
    }

    let settings = read_settings(&root);
    let game_dir = resolve_game_dir(&root, &settings);
    let java = resolve_java(&root, &settings);
    let (memory_auto, memory_mb) = read_memory(&settings);
    let (window_width, window_height) = read_window(&settings);
    let update_mode = read_update_mode(&settings);
    let download_source = read_download_source(&settings);
    let selected_version = game_dir.as_deref().and_then(read_selected_version);

    let mut findings = Vec::new();
    if let Some(dir) = &game_dir {
        findings.push(format!("游戏目录：{}", dir.display()));
    }
    if memory_auto == Some(true) {
        findings.push("内存：自动分配".to_string());
    } else if let Some(mb) = memory_mb {
        findings.push(format!("内存：{:.1} GB", mb as f64 / 1024.0));
    }
    if let Some(java) = &java {
        findings.push(format!("Java：{}", java.display()));
    }
    if let (Some(width), Some(height)) = (window_width, window_height) {
        findings.push(format!("游戏窗口：{width}×{height}"));
    }
    if let Some(mode) = &update_mode {
        findings.push(format!("更新策略：{}", update_mode_label(mode)));
    }
    if let Some(source) = &download_source {
        findings.push(format!("下载源：{}", download_source_label(source)));
    }
    if let Some(version) = &selected_version {
        findings.push(format!("上次启动版本：{version}"));
    }
    findings.push("账号登录信息在 PCL 中加密保存，无法直接迁移，导入后需要重新登录".to_string());

    Ok(PclPreview {
        root: root.to_string_lossy().to_string(),
        game_dir: game_dir.map(|path| path.to_string_lossy().to_string()),
        java_path: java.map(|path| path.to_string_lossy().to_string()),
        memory_auto,
        memory_mb,
        window_width,
        window_height,
        update_mode,
        download_source,
        selected_version,
        findings,
    })
}

/// 只写入确实找到的字段，其余保持启动器当前设置。
pub fn apply(cfg: &mut Config, raw_root: &str) -> Result<PclPreview> {
    let preview = inspect(raw_root)?;
    if let Some(dir) = &preview.game_dir {
        cfg.game_dir = dir.clone();
    }
    if let Some(java) = &preview.java_path {
        cfg.java_path = Some(java.clone());
    }
    if let Some(auto) = preview.memory_auto {
        cfg.memory_auto = auto;
    }
    if let Some(mb) = preview.memory_mb {
        cfg.memory_mb = mb;
    }
    if let (Some(width), Some(height)) = (preview.window_width, preview.window_height) {
        cfg.window_width = width;
        cfg.window_height = height;
    }
    if let Some(mode) = &preview.update_mode {
        cfg.update_mode = mode.clone();
    }
    if let Some(source) = &preview.download_source {
        cfg.download_source = source.clone();
    }
    if let Some(version) = &preview.selected_version {
        cfg.selected_version = Some(version.clone());
    }
    cfg.sync_game_dirs();
    Ok(preview)
}
