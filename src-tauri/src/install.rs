use crate::config::Config;
use crate::download::{self, FileTask, ProgressState};
use crate::models::*;
use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::fs::File;
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tauri::AppHandle;

const MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize)]
pub struct InstalledVersion {
    pub id: String,
    pub kind: String,
    pub jar_size: u64,
    pub has_jar: bool,
    pub last_modified: u64,
    pub java_major: u32,
}

pub async fn fetch_manifest(client: &reqwest::Client, use_mirror: bool) -> Result<VersionManifest> {
    download::get_json(client, MANIFEST_URL, use_mirror).await
}

pub fn list_installed(cfg: &Config) -> Vec<InstalledVersion> {
    let dir = cfg.game_root().join("versions");
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return result;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let json_path = entry.path().join(format!("{id}.json"));
        if !json_path.exists() {
            continue;
        }
        let detail = std::fs::read(&json_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<VersionDetail>(&b).ok());
        let (kind, java_major) = match &detail {
            Some(d) => (d.kind.clone(), d.required_java()),
            None => ("unknown".into(), 8),
        };
        let jar_path = entry.path().join(format!("{id}.jar"));
        let (jar_size, has_jar, modified) = match std::fs::metadata(&jar_path) {
            Ok(meta) => (
                meta.len(),
                true,
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
            Err(_) => (0, false, 0),
        };
        result.push(InstalledVersion {
            id,
            kind,
            jar_size,
            has_jar,
            last_modified: modified,
            java_major,
        });
    }
    result.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    result
}

fn rule_matches(rule: &Rule) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != "windows" {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            // 当前仅支持 64 位 JVM
            if arch == "x86" {
                return false;
            }
        }
    }
    if let Some(features) = &rule.features {
        for (key, expected) in features {
            let actual = match key.as_str() {
                "has_custom_resolution" => true,
                "is_demo_user" => false,
                "has_quick_plays_support" => false,
                "is_quick_play_singleplayer" => false,
                "is_quick_play_multiplayer" => false,
                "is_quick_play_realms" => false,
                _ => false,
            };
            if actual != *expected {
                return false;
            }
        }
    }
    true
}

pub fn rules_allow(rules: Option<&Vec<Rule>>) -> bool {
    match rules {
        None => true,
        Some(list) => {
            let mut allowed = false;
            for rule in list {
                if rule_matches(rule) {
                    allowed = rule.action == "allow";
                }
            }
            allowed
        }
    }
}

fn maven_parts(name: &str) -> Option<(String, String, String, Option<String>, String)> {
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e.to_string()),
        None => (name, "jar".to_string()),
    };
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].to_string();
    let artifact = parts[1].to_string();
    let version = parts[2].to_string();
    let classifier = parts.get(3).map(|s| s.to_string());
    Some((group, artifact, version, classifier, ext))
}

fn maven_path(name: &str) -> Option<String> {
    let (group, artifact, version, classifier, ext) = maven_parts(name)?;
    let file = match classifier {
        Some(c) => format!("{artifact}-{version}-{c}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!(
        "{}/{}/{}/{}",
        group.replace('.', "/"),
        artifact,
        version,
        file
    ))
}

fn library_is_native(lib: &Library) -> bool {
    // 旧版清单用 natives 映射 + extract 表示需要解压；
    // 1.19 之后 natives 作为普通 classifier 库，直接进 classpath。
    lib.natives
        .as_ref()
        .map(|n| n.contains_key("windows"))
        .unwrap_or(false)
        || lib.extract.is_some()
}

fn library_artifact(lib: &Library) -> Option<(DownloadRef, String)> {
    if let Some(downloads) = &lib.downloads {
        if let Some(artifact) = &downloads.artifact {
            let path = artifact
                .path
                .clone()
                .or_else(|| maven_path(&lib.name))
                .unwrap_or_default();
            return Some((artifact.clone(), path));
        }
    }
    None
}

fn library_native_download(lib: &Library) -> Option<(DownloadRef, String)> {
    let downloads = lib.downloads.as_ref()?;
    let (group, artifact, version, name_classifier, _) = maven_parts(&lib.name)?;
    let classifier = lib
        .natives
        .as_ref()
        .and_then(|n| n.get("windows").cloned())
        .or_else(|| name_classifier.filter(|c| c.starts_with("natives")))?;
    if let Some(classifiers) = &downloads.classifiers {
        if let Some(dl) = classifiers.get(&classifier) {
            let file = format!("{artifact}-{version}-{classifier}.jar");
            let path = format!(
                "{}/{}/{}/{}",
                group.replace('.', "/"),
                artifact,
                version,
                file
            );
            return Some((dl.clone(), path));
        }
    }
    // 新版清单里 natives 直接作为 artifact 提供
    downloads.artifact.as_ref().map(|dl| {
        let path = dl
            .path
            .clone()
            .or_else(|| maven_path(&lib.name))
            .unwrap_or_default();
        (dl.clone(), path)
    })
}

fn extract_natives(jar: &Path, dest: &Path, exclude: &[String]) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let file = File::open(jar).with_context(|| format!("无法打开 {}", jar.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let rel = rel.to_path_buf();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if exclude.iter().any(|ex| {
            let ex = ex.trim_end_matches('/');
            rel_str == ex || rel_str.starts_with(&format!("{ex}/"))
        }) {
            continue;
        }
        let out_path = dest.join(&rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        copy(&mut entry, &mut out)?;
    }
    Ok(())
}

pub async fn install_version(
    app: &AppHandle,
    client: &reqwest::Client,
    cfg: &Config,
    version_id: &str,
    progress: Arc<ProgressState>,
) -> Result<InstalledVersion> {
    let use_mirror = cfg.use_mirror();
    let game_root = cfg.game_root();
    let version_dir = game_root.join("versions").join(version_id);
    tokio::fs::create_dir_all(&version_dir).await?;

    progress.set_message("正在获取版本信息…");
    let manifest = fetch_manifest(client, use_mirror).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| anyhow!("版本 {version_id} 不存在"))?;

    let raw = download::get_bytes(client, &entry.url, use_mirror).await?;
    let detail: VersionDetail = serde_json::from_slice(&raw).context("解析版本 JSON 失败")?;
    if let Some(parent) = &detail.inherits_from {
        bail!("当前版本依赖 {parent}，原版安装暂不支持继承版本");
    }

    let version_json = version_dir.join(format!("{version_id}.json"));
    tokio::fs::write(&version_json, &raw).await?;

    // 1) 客户端 jar、依赖库、本地库
    let mut tasks: Vec<FileTask> = Vec::new();
    if let Some(client_dl) = detail.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        tasks.push(FileTask {
            url: client_dl.url.clone(),
            dest: version_dir.join(format!("{version_id}.jar")),
            sha1: Some(client_dl.sha1.clone()),
            size: client_dl.size,
            label: format!("{version_id}.jar"),
        });
    }

    let mut native_jars: Vec<(PathBuf, Vec<String>)> = Vec::new();
    for lib in &detail.libraries {
        if !rules_allow(lib.rules.as_ref()) {
            continue;
        }
        let native = library_is_native(lib);
        let chosen = if native {
            library_native_download(lib)
        } else {
            library_artifact(lib)
        };
        if let Some((dl, path)) = chosen {
            if path.is_empty() {
                continue;
            }
            let dest = game_root.join("libraries").join(path.replace('/', "\\"));
            tasks.push(FileTask {
                url: dl.url,
                dest: dest.clone(),
                sha1: Some(dl.sha1),
                size: dl.size,
                label: dest
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            });
            if native {
                let exclude = lib
                    .extract
                    .as_ref()
                    .and_then(|e| e.exclude.clone())
                    .unwrap_or_else(|| vec!["META-INF/".to_string()]);
                native_jars.push((dest, exclude));
            }
        }
    }

    progress.set_message("正在下载游戏核心与依赖库…");
    download::run_tasks(
        app,
        client,
        tasks,
        cfg.concurrency as usize,
        use_mirror,
        progress.clone(),
    )
    .await?;

    // 2) 资源索引与资源文件
    if let Some(index_ref) = &detail.asset_index {
        progress.set_message("正在获取资源索引…");
        let raw = download::get_bytes(client, &index_ref.url, use_mirror).await?;
        let index: AssetIndexFile =
            serde_json::from_slice(&raw).context("解析资源索引失败")?;
        let index_path = game_root
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", index_ref.id));
        if let Some(parent) = index_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&index_path, &raw).await?;

        let mut asset_tasks = Vec::new();
        for (name, obj) in &index.objects {
            if obj.hash.len() < 2 {
                continue;
            }
            let dest = game_root
                .join("assets")
                .join("objects")
                .join(&obj.hash[..2])
                .join(&obj.hash);
            asset_tasks.push(FileTask {
                url: format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &obj.hash[..2],
                    obj.hash
                ),
                dest,
                sha1: Some(obj.hash.clone()),
                size: obj.size,
                label: name.clone(),
            });
        }
        progress.set_message(format!("正在下载 {} 个资源文件…", asset_tasks.len()));
        download::run_tasks(
            app,
            client,
            asset_tasks,
            cfg.concurrency as usize,
            use_mirror,
            progress.clone(),
        )
        .await?;
    }

    // 3) 日志配置
    if let Some(logging) = detail.logging.as_ref().and_then(|l| l.client.as_ref()) {
        let dest = game_root
            .join("assets")
            .join("log_configs")
            .join(&logging.file.url.rsplit('/').next().unwrap_or("log4j.xml"));
        let tasks = vec![FileTask {
            url: logging.file.url.clone(),
            dest,
            sha1: Some(logging.file.sha1.clone()),
            size: logging.file.size,
            label: "日志配置".into(),
        }];
        download::run_tasks(app, client, tasks, 1, use_mirror, progress.clone()).await?;
    }

    // 4) 解压旧版本地库；新版 natives 直接走 classpath
    progress.set_message("正在准备本地库…");
    let natives_dir = version_dir.join("natives");
    if natives_dir.exists() {
        let _ = std::fs::remove_dir_all(&natives_dir);
    }
    tokio::fs::create_dir_all(&natives_dir).await?;
    for sub in ["java", "jna", "lwjgl", "netty"] {
        tokio::fs::create_dir_all(natives_dir.join(sub)).await?;
    }
    if !native_jars.is_empty() {
        for (jar, exclude) in &native_jars {
            let jar = jar.clone();
            let jar_label = jar.clone();
            let dest = natives_dir.clone();
            let exclude = exclude.clone();
            tokio::task::spawn_blocking(move || extract_natives(&jar, &dest, &exclude))
                .await?
                .with_context(|| format!("解压本地库失败：{}", jar_label.display()))?;
        }
    }

    progress.set_message("安装完成");
    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(app, &progress);

    let jar_path = version_dir.join(format!("{version_id}.jar"));
    let kind = detail.kind.clone();
    let java_major = detail.required_java();
    Ok(InstalledVersion {
        id: version_id.to_string(),
        kind,
        jar_size: std::fs::metadata(&jar_path).map(|m| m.len()).unwrap_or(0),
        has_jar: jar_path.exists(),
        last_modified: std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        java_major,
    })
}

pub fn is_installed(cfg: &Config, version_id: &str) -> bool {
    cfg.game_root()
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.json"))
        .exists()
}

/// 供启动流程复用：读取已安装版本的详情
pub fn read_installed_detail(cfg: &Config, version_id: &str) -> Result<VersionDetail> {
    let path = cfg
        .game_root()
        .join("versions")
        .join(version_id)
        .join(format!("{version_id}.json"));
    let bytes = std::fs::read(&path)?;
    let detail = serde_json::from_slice::<VersionDetail>(&bytes)?;
    Ok(detail)
}

pub fn library_file_paths(cfg: &Config, detail: &VersionDetail) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for lib in &detail.libraries {
        if !rules_allow(lib.rules.as_ref()) || library_is_native(lib) {
            continue;
        }
        let path = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.path.clone())
            .or_else(|| maven_path(&lib.name));
        if let Some(rel) = path {
            paths.push(cfg.game_root().join("libraries").join(rel.replace('/', "\\")));
        }
    }
    paths
}

pub fn merged_classpath_entries(cfg: &Config, detail: &VersionDetail) -> Vec<PathBuf> {
    let mut entries = library_file_paths(cfg, detail);
    entries.push(
        cfg.game_root()
            .join("versions")
            .join(&detail.id)
            .join(format!("{}.jar", detail.id)),
    );
    entries
}

pub async fn remove_version(cfg: &Config, version_id: &str) -> Result<()> {
    let dir = cfg.game_root().join("versions").join(version_id);
    if dir.exists() {
        tokio::fs::remove_dir_all(&dir).await?;
    }
    Ok(())
}

pub fn resolve_version_dir(cfg: &Config, version_id: &str) -> PathBuf {
    cfg.game_root().join("versions").join(version_id)
}

pub fn natives_dir(cfg: &Config, version_id: &str) -> PathBuf {
    resolve_version_dir(cfg, version_id).join("natives")
}
