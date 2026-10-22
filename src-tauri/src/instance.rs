use crate::config::{self, Config};
use crate::install;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn meta_path() -> PathBuf {
    config::data_dir().join("instances.json")
}

/// 每个版本独立的元数据与启动覆盖项，按版本 ID 存储。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InstanceMeta {
    pub display_name: Option<String>,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub favorite: bool,
    pub memory_mb: Option<u32>,
    pub java_path: Option<String>,
    pub java_args: Option<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub fullscreen: Option<bool>,
}

fn load_map() -> HashMap<String, InstanceMeta> {
    std::fs::read(meta_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_map(map: &HashMap<String, InstanceMeta>) -> Result<()> {
    std::fs::create_dir_all(config::data_dir())?;
    std::fs::write(meta_path(), serde_json::to_vec_pretty(map)?)?;
    Ok(())
}

fn meta_key(cfg: &Config, version_id: &str) -> String {
    format!(
        "{}::{}",
        cfg.game_dir
            .trim_end_matches(['\\', '/'])
            .to_lowercase(),
        version_id
    )
}

pub fn get(cfg: &Config, version_id: &str) -> InstanceMeta {
    let map = load_map();
    map.get(&meta_key(cfg, version_id))
        .cloned()
        // 兼容旧版只按版本 ID 存储的数据
        .or_else(|| map.get(version_id).cloned())
        .unwrap_or_default()
}

pub fn save(cfg: &Config, version_id: &str, meta: &InstanceMeta) -> Result<()> {
    let mut map = load_map();
    map.insert(meta_key(cfg, version_id), meta.clone());
    save_map(&map)
}

pub fn remove(cfg: &Config, version_id: &str) -> Result<()> {
    let mut map = load_map();
    let mut changed = map.remove(&meta_key(cfg, version_id)).is_some();
    changed |= map.remove(version_id).is_some();
    if changed {
        save_map(&map)?;
    }
    Ok(())
}

/// 只返回当前游戏目录下的版本元数据，前端按版本 ID 取用。
pub fn all(cfg: &Config) -> HashMap<String, InstanceMeta> {
    let prefix = format!(
        "{}::",
        cfg.game_dir
            .trim_end_matches(['\\', '/'])
            .to_lowercase()
    );
    let mut result = HashMap::new();
    for (key, meta) in load_map() {
        if let Some(version_id) = key.strip_prefix(&prefix) {
            result.insert(version_id.to_string(), meta);
        } else if !key.contains("::") {
            // 旧格式数据直接按 ID 兼容
            result.insert(key, meta);
        }
    }
    result
}

/// 把版本级覆盖项叠到全局配置上，供启动时使用。
pub fn apply(cfg: &Config, version_id: &str) -> Config {
    let meta = get(cfg, version_id);
    let mut merged = cfg.clone();
    if let Some(memory) = meta.memory_mb {
        merged.memory_mb = memory.max(512);
    }
    if let Some(path) = meta.java_path.filter(|path| !path.trim().is_empty()) {
        merged.java_path = Some(path);
    }
    if let Some(args) = meta.java_args {
        merged.java_args = args;
    }
    if let Some(width) = meta.window_width.filter(|value| *value >= 320) {
        merged.window_width = width;
    }
    if let Some(height) = meta.window_height.filter(|value| *value >= 240) {
        merged.window_height = height;
    }
    if let Some(fullscreen) = meta.fullscreen {
        merged.fullscreen = fullscreen;
    }
    merged
}

#[derive(Debug, Clone, Serialize)]
pub struct InstanceFile {
    pub name: String,
    pub display_name: String,
    pub size: u64,
    pub modified: u64,
    pub enabled: bool,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct InstanceCounts {
    pub mods: u32,
    pub resourcepacks: u32,
    pub shaderpacks: u32,
    pub saves: u32,
    pub screenshots: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstancePaths {
    pub version: String,
    pub mods: String,
    pub resourcepacks: String,
    pub shaderpacks: String,
    pub saves: String,
    pub screenshots: String,
    pub game_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstanceInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub favorite: bool,
    pub game_version: String,
    pub loader: String,
    pub kind: String,
    pub java_major: u32,
    pub path: String,
    pub installed_at: u64,
    pub memory_mb: Option<u32>,
    pub java_path: Option<String>,
    pub java_args: Option<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub fullscreen: Option<bool>,
    pub counts: InstanceCounts,
}

fn content_subdir(kind: &str) -> Result<&'static str> {
    match kind {
        "mods" => Ok("mods"),
        "resourcepacks" => Ok("resourcepacks"),
        "shaderpacks" => Ok("shaderpacks"),
        "saves" => Ok("saves"),
        "screenshots" => Ok("screenshots"),
        _ => bail!("未知的内容分类：{kind}"),
    }
}

pub fn content_dir(cfg: &Config, version_id: &str, kind: &str) -> Result<PathBuf> {
    Ok(cfg.launch_dir(version_id).join(content_subdir(kind)?))
}

pub fn paths(cfg: &Config, version_id: &str) -> InstancePaths {
    let game_dir = cfg.launch_dir(version_id);
    for name in ["mods", "resourcepacks", "shaderpacks", "saves", "screenshots"] {
        let _ = std::fs::create_dir_all(game_dir.join(name));
    }
    InstancePaths {
        version: install::resolve_version_dir(cfg, version_id)
            .to_string_lossy()
            .to_string(),
        mods: game_dir.join("mods").to_string_lossy().to_string(),
        resourcepacks: game_dir.join("resourcepacks").to_string_lossy().to_string(),
        shaderpacks: game_dir.join("shaderpacks").to_string_lossy().to_string(),
        saves: game_dir.join("saves").to_string_lossy().to_string(),
        screenshots: game_dir.join("screenshots").to_string_lossy().to_string(),
        game_dir: game_dir.to_string_lossy().to_string(),
    }
}

fn modified_secs(path: &Path) -> u64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn list_files(cfg: &Config, version_id: &str, kind: &str) -> Result<Vec<InstanceFile>> {
    let dir = content_dir(cfg, version_id, kind)?;
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if kind == "saves" && !is_dir {
            continue;
        }
        let enabled = !name.ends_with(".disabled");
        let display_name = name.trim_end_matches(".disabled").to_string();
        let size = if is_dir {
            0
        } else {
            std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0)
        };
        files.push(InstanceFile {
            name,
            display_name,
            size,
            modified: modified_secs(&path),
            enabled,
            is_dir,
        });
    }
    files.sort_by(|a, b| b.enabled.cmp(&a.enabled).then(a.display_name.cmp(&b.display_name)));
    Ok(files)
}

fn safe_name(name: &str) -> Result<&str> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == ".." {
        bail!("无效的文件名");
    }
    Ok(name)
}

pub fn toggle_file(cfg: &Config, version_id: &str, kind: &str, name: &str) -> Result<()> {
    if !matches!(kind, "mods" | "resourcepacks" | "shaderpacks") {
        bail!("该分类不支持启用/禁用");
    }
    let name = safe_name(name)?;
    let dir = content_dir(cfg, version_id, kind)?;
    let path = dir.join(name);
    if !path.exists() {
        bail!("文件不存在：{name}");
    }
    let target = if name.ends_with(".disabled") {
        dir.join(name.trim_end_matches(".disabled"))
    } else {
        dir.join(format!("{name}.disabled"))
    };
    if target.exists() {
        bail!("目标文件已存在");
    }
    std::fs::rename(&path, &target).context("重命名失败，游戏可能正在运行")?;
    Ok(())
}

pub fn delete_file(cfg: &Config, version_id: &str, kind: &str, name: &str) -> Result<()> {
    let name = safe_name(name)?;
    let dir = content_dir(cfg, version_id, kind)?;
    let path = dir.join(name);
    if !path.exists() {
        bail!("文件不存在：{name}");
    }
    if path.is_dir() {
        std::fs::remove_dir_all(&path)?;
    } else {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

fn unique_target(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let extension = path.extension().map(|value| value.to_string_lossy().to_string());
    for index in 1..1000 {
        let next = match &extension {
            Some(ext) => format!("{stem} ({index}).{ext}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = dir.join(next);
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(name)
}

fn copy_dir_all(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

pub fn import_paths(
    cfg: &Config,
    version_id: &str,
    kind: &str,
    paths: &[String],
) -> Result<u32> {
    let dir = content_dir(cfg, version_id, kind)?;
    std::fs::create_dir_all(&dir)?;
    let mut count = 0;
    for raw in paths {
        let src = PathBuf::from(raw);
        if !src.exists() {
            continue;
        }
        let name = src
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let target = unique_target(&dir, &name);
        if src.is_dir() {
            copy_dir_all(&src, &target)?;
        } else {
            std::fs::copy(&src, &target)?;
        }
        count += 1;
    }
    Ok(count)
}

pub fn info(cfg: &Config, version_id: &str) -> Result<InstanceInfo> {
    let meta = get(cfg, version_id);
    let (game_version, loader) = install::instance_context(cfg, version_id)?;
    let merged = install::resolve_detail(cfg, version_id)?;
    let installed = install::list_installed(cfg);
    let entry = installed
        .iter()
        .find(|item| item.id == version_id)
        .ok_or_else(|| anyhow::anyhow!("版本未安装：{version_id}"))?;

    let counts = InstanceCounts {
        mods: list_files(cfg, version_id, "mods")
            .map(|list| list.len() as u32)
            .unwrap_or(0),
        resourcepacks: list_files(cfg, version_id, "resourcepacks")
            .map(|list| list.len() as u32)
            .unwrap_or(0),
        shaderpacks: list_files(cfg, version_id, "shaderpacks")
            .map(|list| list.len() as u32)
            .unwrap_or(0),
        saves: list_files(cfg, version_id, "saves")
            .map(|list| list.len() as u32)
            .unwrap_or(0),
        screenshots: list_files(cfg, version_id, "screenshots")
            .map(|list| list.len() as u32)
            .unwrap_or(0),
    };

    Ok(InstanceInfo {
        id: version_id.to_string(),
        display_name: meta
            .display_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| version_id.to_string()),
        description: meta.description.clone(),
        icon: if meta.icon.is_empty() {
            "auto".into()
        } else {
            meta.icon.clone()
        },
        category: if meta.category.is_empty() {
            "auto".into()
        } else {
            meta.category.clone()
        },
        favorite: meta.favorite,
        game_version,
        loader,
        kind: merged.detail.kind.clone(),
        java_major: merged.detail.required_java(),
        path: install::resolve_version_dir(cfg, version_id)
            .to_string_lossy()
            .to_string(),
        installed_at: entry.last_modified,
        memory_mb: meta.memory_mb,
        java_path: meta.java_path,
        java_args: meta.java_args,
        window_width: meta.window_width,
        window_height: meta.window_height,
        fullscreen: meta.fullscreen,
        counts,
    })
}
