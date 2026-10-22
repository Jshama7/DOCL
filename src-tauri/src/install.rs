use crate::config::{self, Config};
use crate::download::{self, FileTask, ProgressState};
use crate::launch;
use crate::models::*;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
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
        let resolved = resolve_detail(cfg, &id).ok();
        let (kind, java_major) = match &resolved {
            Some(merged) => (merged.detail.kind.clone(), merged.detail.required_java()),
            None => ("unknown".into(), 8),
        };
        let jar_path = match resolved.as_ref().and_then(|m| m.jar_id.as_ref()) {
            Some(jar_id) => dir.join(jar_id).join(format!("{jar_id}.jar")),
            None => entry.path().join(format!("{id}.jar")),
        };
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
    // Fabric / Quilt：name + url 形式
    if let Some(base) = &lib.url {
        let path = maven_path(&lib.name)?;
        let url = format!("{}/{}", base.trim_end_matches('/'), path);
        return Some((
            DownloadRef {
                sha1: lib.sha1.clone().unwrap_or_default(),
                size: lib.size.unwrap_or(0),
                url,
                path: Some(path.clone()),
            },
            path,
        ));
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

#[derive(Debug, Clone)]
pub struct MergedDetail {
    pub detail: VersionDetail,
    pub jar_id: Option<String>,
    /// 子版本 → 根版本
    pub chain: Vec<String>,
}

fn merge_child(base: &mut VersionDetail, child: &VersionDetail) {
    if !child.kind.is_empty() {
        base.kind = child.kind.clone();
    }
    if let Some(main_class) = &child.main_class {
        base.main_class = Some(main_class.clone());
    }
    if let Some(index) = &child.asset_index {
        base.asset_index = Some(index.clone());
    }
    if let Some(downloads) = &child.downloads {
        base.downloads = Some(downloads.clone());
    }
    if let Some(java) = &child.java_version {
        base.java_version = Some(java.clone());
    }
    if let Some(logging) = &child.logging {
        base.logging = Some(logging.clone());
    }
    if let Some(assets) = &child.assets {
        base.assets = Some(assets.clone());
    }

    for lib in &child.libraries {
        if let Some(pos) = base.libraries.iter().position(|l| l.name == lib.name) {
            base.libraries[pos] = lib.clone();
        } else {
            base.libraries.push(lib.clone());
        }
    }

    let mut args = base.arguments.take().unwrap_or(Arguments {
        game: Vec::new(),
        jvm: Vec::new(),
    });
    if let Some(child_args) = &child.arguments {
        args.game.extend(child_args.game.clone());
        args.jvm.extend(child_args.jvm.clone());
    }
    if !args.game.is_empty() || !args.jvm.is_empty() {
        base.arguments = Some(args);
    }
    if base.minecraft_arguments.is_none() {
        base.minecraft_arguments = child.minecraft_arguments.clone();
    }
}

pub fn resolve_detail(cfg: &Config, version_id: &str) -> Result<MergedDetail> {
    let mut chain: Vec<VersionDetail> = Vec::new();
    let mut visited: Vec<String> = Vec::new();
    let mut current = Some(version_id.to_string());
    let mut jar_id: Option<String> = None;

    while let Some(id) = current {
        if visited.contains(&id) {
            bail!("版本继承出现循环：{id}");
        }
        visited.push(id.clone());
        let detail = read_installed_detail(cfg, &id)?;
        if jar_id.is_none()
            && detail
                .downloads
                .as_ref()
                .and_then(|d| d.client.as_ref())
                .is_some()
        {
            jar_id = Some(id.clone());
        }
        current = detail.inherits_from.clone();
        chain.push(detail);
    }

    let mut merged = chain
        .last()
        .cloned()
        .ok_or_else(|| anyhow!("版本 {version_id} 内容为空"))?;
    for child in chain.iter().rev().skip(1) {
        merge_child(&mut merged, child);
    }
    merged.id = version_id.to_string();
    merged.inherits_from = None;
    let ids = chain.into_iter().map(|d| d.id).collect();
    Ok(MergedDetail {
        detail: merged,
        jar_id,
        chain: ids,
    })
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

pub fn merged_classpath_entries(
    cfg: &Config,
    detail: &VersionDetail,
    jar_id: Option<&str>,
) -> Vec<PathBuf> {
    let mut entries = library_file_paths(cfg, detail);
    if let Some(jar) = jar_id {
        entries.push(
            cfg.game_root()
                .join("versions")
                .join(jar)
                .join(format!("{jar}.jar")),
        );
    }
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

/* ---------- 模组加载器 ---------- */

#[derive(Debug, Clone, Serialize)]
pub struct LoaderOption {
    pub loader: String,
    pub label: String,
    pub version: Option<String>,
    pub installed: bool,
    pub installed_id: Option<String>,
    pub supported: bool,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BmclForgePromo {
    name: String,
    build: BmclForgeBuild,
}

#[derive(Debug, Clone, Deserialize)]
struct BmclForgeBuild {
    version: String,
}

async fn forge_recommended(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<String> {
    let list: Vec<BmclForgePromo> = download::get_json(
        client,
        "https://bmclapi2.bangbang93.com/forge/promos",
        false,
    )
    .await?;
    let recommended = format!("{game_version}-recommended");
    let latest = format!("{game_version}-latest");
    list.iter()
        .find(|entry| entry.name == recommended)
        .or_else(|| list.iter().find(|entry| entry.name == latest))
        .map(|entry| entry.build.version.clone())
        .ok_or_else(|| anyhow!("Forge 没有适配 {game_version} 的版本"))
}

fn xml_versions(text: &str) -> Vec<String> {
    text.split("<version>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</version>").next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn version_key(value: &str) -> Vec<u64> {
    value
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

async fn fetch_maven_metadata(client: &reqwest::Client, path: &str) -> Result<Vec<String>> {
    let mirrors = [
        format!("https://bmclapi2.bangbang93.com/maven/{path}"),
        format!("https://maven.neoforged.net/releases/{path}"),
    ];
    for url in mirrors {
        if let Ok(bytes) = download::get_bytes(client, &url, false).await {
            let text = String::from_utf8_lossy(&bytes);
            let versions = xml_versions(&text);
            if !versions.is_empty() {
                return Ok(versions);
            }
        }
    }
    bail!("无法获取 NeoForge 版本列表")
}

/// 返回 (安装器坐标版本, 下载地址, 文件名)
async fn neoforge_installer(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<(String, String, String)> {
    if game_version == "1.20.1" {
        let versions =
            fetch_maven_metadata(client, "net/neoforged/forge/maven-metadata.xml").await?;
        let pick = versions
            .iter()
            .filter(|v| v.starts_with("1.20.1-") && !v.contains("beta"))
            .max_by_key(|v| version_key(v))
            .ok_or_else(|| anyhow!("NeoForge 没有适配 1.20.1 的版本"))?
            .clone();
        let file = format!("forge-{pick}-installer.jar");
        return Ok((
            pick.clone(),
            format!(
                "https://bmclapi2.bangbang93.com/maven/net/neoforged/forge/{pick}/{file}"
            ),
            file,
        ));
    }

    let prefix = game_version
        .strip_prefix("1.")
        .unwrap_or(game_version)
        .to_string();
    let mut versions =
        fetch_maven_metadata(client, "net/neoforged/neoforge/maven-metadata.xml").await?;
    versions.retain(|v| {
        v.starts_with(&format!("{prefix}."))
            && !v.contains('-')
            && v.split('.').all(|part| part.chars().all(|c| c.is_ascii_digit()))
    });
    let pick = versions
        .into_iter()
        .max_by_key(|v| version_key(v))
        .ok_or_else(|| anyhow!("NeoForge 没有适配 {game_version} 的版本"))?;
    let file = format!("neoforge-{pick}-installer.jar");
    Ok((
        pick.clone(),
        format!(
            "https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/{pick}/{file}"
        ),
        file,
    ))
}

async fn forge_installer(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<(String, String, String)> {
    let version = forge_recommended(client, game_version).await?;
    let coordinate = format!("{game_version}-{version}");
    let file = format!("forge-{coordinate}-installer.jar");
    Ok((
        version,
        format!(
            "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{coordinate}/{file}"
        ),
        file,
    ))
}

async fn install_forge_like(
    app: &AppHandle,
    client: &reqwest::Client,
    cfg: &Config,
    game_version: &str,
    loader: &str,
    progress: Arc<ProgressState>,
) -> Result<InstalledVersion> {
    let required = read_installed_detail(cfg, game_version)?.required_java();
    let java = launch::find_java(cfg, required)?;

    progress.set_message(format!("正在获取 {loader} 安装器地址…"));
    let (version, url, file_name) = if loader == "neoforge" {
        neoforge_installer(client, game_version).await?
    } else {
        forge_installer(client, game_version).await?
    };

    let cache = config::data_dir().join("installers");
    tokio::fs::create_dir_all(&cache).await?;
    let installer_path = cache.join(&file_name);
    if !installer_path.exists() {
        progress.set_message(format!("正在下载 {loader} 安装器…"));
        let bytes = download::get_bytes(client, &url, false)
            .await
            .with_context(|| format!("下载 {loader} 安装器失败"))?;
        tokio::fs::write(&installer_path, bytes).await?;
    }

    progress.set_message(format!(
        "正在运行 {loader} 安装器（可能需要几分钟）…"
    ));
    progress.files_total.store(1, Ordering::Relaxed);
    progress.files_done.store(0, Ordering::Relaxed);
    progress.active.store(true, Ordering::Relaxed);
    download::emit_now(app, &progress);

    let mut command = tokio::process::Command::new(&java.path);
    command
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(cfg.game_root());
    if cfg.use_mirror() {
        command
            .arg("--mirror")
            .arg("https://bmclapi2.bangbang93.com/maven");
    }
    command
        .current_dir(cfg.game_root())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().context("启动 Forge/NeoForge 安装器失败")?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let read_stream = |stream: Option<tokio::process::ChildStdout>,
                       progress: Arc<ProgressState>| async move {
        if let Some(stream) = stream {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let text = line.trim();
                if !text.is_empty() {
                    progress.set_message(text.chars().take(120).collect::<String>());
                }
            }
        }
    };
    let read_err = |stream: Option<tokio::process::ChildStderr>,
                    progress: Arc<ProgressState>| async move {
        if let Some(stream) = stream {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let text = line.trim();
                if !text.is_empty() {
                    progress.set_message(text.chars().take(120).collect::<String>());
                }
            }
        }
    };

    let out_task = tokio::spawn(read_stream(stdout, progress.clone()));
    let err_task = tokio::spawn(read_err(stderr, progress.clone()));

    let status = tokio::time::timeout(std::time::Duration::from_secs(900), child.wait())
        .await
        .map_err(|_| anyhow!("{loader} 安装器运行超时"))??;
    let _ = out_task.await;
    let _ = err_task.await;
    if !status.success() {
        bail!(
            "{} 安装器退出码：{}，请查看安装器输出或改用离线安装",
            loader,
            status.code().unwrap_or(-1)
        );
    }

    progress.files_done.store(1, Ordering::Relaxed);
    progress.set_message(format!("{loader} 安装完成"));

    let target = installed_loaders(cfg, game_version)
        .into_iter()
        .find(|(name, _)| name == loader)
        .map(|(_, id)| id)
        .ok_or_else(|| anyhow!("安装器已结束，但没有找到 {loader} 版本，请检查游戏目录"))?;

    if let Some(parent_natives) = {
        let path = natives_dir(cfg, game_version);
        path.exists().then_some(path)
    } {
        let dest = natives_dir(cfg, &target);
        if !dest.exists() {
            tokio::fs::create_dir_all(&dest).await?;
            let source = parent_natives;
            let dest_clone = dest.clone();
            tokio::task::spawn_blocking(move || copy_dir_all(&source, &dest_clone)).await??;
        }
    }

    let detail = resolve_detail(cfg, &target)?;
    let jar_size = detail
        .jar_id
        .as_ref()
        .and_then(|jar_id| {
            std::fs::metadata(
                cfg.game_root()
                    .join("versions")
                    .join(jar_id)
                    .join(format!("{jar_id}.jar")),
            )
            .ok()
        })
        .map(|m| m.len())
        .unwrap_or(0);
    let _ = version;
    Ok(InstalledVersion {
        id: target,
        kind: "modded".into(),
        jar_size,
        has_jar: true,
        last_modified: std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        java_major: detail.detail.required_java(),
    })
}

#[derive(Debug, Clone, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderVersion,
}

#[derive(Debug, Clone, Deserialize)]
struct FabricLoaderVersion {
    version: String,
    #[serde(default)]
    stable: bool,
}

fn loader_list_url(loader: &str, game_version: &str) -> Option<String> {
    match loader {
        "fabric" => Some(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{game_version}"
        )),
        "quilt" => Some(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{game_version}"
        )),
        _ => None,
    }
}

fn loader_profile_url(loader: &str, game_version: &str, loader_version: &str) -> Option<String> {
    match loader {
        "fabric" => Some(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/profile/json"
        )),
        "quilt" => Some(format!(
            "https://meta.quiltmc.org/v3/versions/loader/{game_version}/{loader_version}/profile/json"
        )),
        _ => None,
    }
}

async fn pick_loader_version(
    client: &reqwest::Client,
    cfg: &Config,
    loader: &str,
    game_version: &str,
) -> Result<String> {
    if loader == "forge" {
        return forge_recommended(client, game_version).await;
    }
    if loader == "neoforge" {
        let (version, _, _) = neoforge_installer(client, game_version).await?;
        return Ok(version);
    }
    let url = loader_list_url(loader, game_version)
        .ok_or_else(|| anyhow!("不支持的加载器：{loader}"))?;
    let list: Vec<FabricLoaderEntry> =
        download::get_json(client, &url, cfg.use_mirror()).await?;
    let stable = list.iter().find(|entry| entry.loader.stable);
    let pick = stable.or_else(|| list.first());
    pick.map(|entry| entry.loader.version.clone())
        .ok_or_else(|| anyhow!("{loader} 没有适配 {game_version} 的加载器版本"))
}

fn installed_loaders(cfg: &Config, game_version: &str) -> Vec<(String, String)> {
    let dir = cfg.game_root().join("versions");
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let id = entry.file_name().to_string_lossy().to_string();
        let json = entry.path().join(format!("{id}.json"));
        let Ok(bytes) = std::fs::read(&json) else {
            continue;
        };
        let Ok(detail) = serde_json::from_slice::<VersionDetail>(&bytes) else {
            continue;
        };
        if detail.inherits_from.as_deref() != Some(game_version) {
            continue;
        }
        let loader = if id.starts_with("fabric-loader") {
            "fabric"
        } else if id.starts_with("quilt-loader") {
            "quilt"
        } else if id.contains("neoforge") {
            "neoforge"
        } else if id.contains("forge") {
            "forge"
        } else {
            continue;
        };
        result.push((loader.to_string(), id));
    }
    result
}

pub async fn loader_options(
    client: &reqwest::Client,
    cfg: &Config,
    game_version: &str,
) -> Vec<LoaderOption> {
    let installed = installed_loaders(cfg, game_version);
    let loaders = [
        ("fabric", "Fabric", true),
        ("quilt", "Quilt", true),
        ("forge", "Forge", true),
        ("neoforge", "NeoForge", true),
        ("optifine", "OptiFine", false),
    ];
    let mut result = Vec::new();
    for (loader, label, supported) in loaders {
        let version = if supported {
            pick_loader_version(client, cfg, loader, game_version)
                .await
                .ok()
        } else {
            None
        };
        let found = installed.iter().find(|(name, _)| name == loader);
        let note = if !supported {
            "暂未支持"
        } else if found.is_some() {
            "已安装"
        } else if version.is_none() {
            "无法获取版本"
        } else {
            "可以添加"
        };
        result.push(LoaderOption {
            loader: loader.to_string(),
            label: label.to_string(),
            version,
            installed: found.is_some(),
            installed_id: found.map(|(_, id)| id.clone()),
            supported,
            note: note.to_string(),
        });
    }
    result
}

fn copy_dir_all(source: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(source)?.flatten() {
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub async fn install_loader(
    app: &AppHandle,
    client: &reqwest::Client,
    cfg: &Config,
    game_version: &str,
    loader: &str,
    progress: Arc<ProgressState>,
) -> Result<InstalledVersion> {
    let use_mirror = cfg.use_mirror();
    if !is_installed(cfg, game_version) {
        progress.set_message(format!("正在安装原版 {game_version}…"));
        install_version(app, client, cfg, game_version, progress.clone()).await?;
    }

    if loader == "forge" || loader == "neoforge" {
        return install_forge_like(app, client, cfg, game_version, loader, progress).await;
    }

    progress.set_message(format!("正在获取 {loader} 加载器版本…"));
    let loader_version = pick_loader_version(client, cfg, loader, game_version).await?;
    let profile_url = loader_profile_url(loader, game_version, &loader_version)
        .ok_or_else(|| anyhow!("不支持的加载器：{loader}"))?;
    let raw = download::get_bytes(client, &profile_url, use_mirror).await?;
    let profile: VersionDetail =
        serde_json::from_slice(&raw).context("解析加载器配置失败")?;
    let loader_id = profile.id.clone();
    let version_dir = cfg.game_root().join("versions").join(&loader_id);
    tokio::fs::create_dir_all(&version_dir).await?;
    tokio::fs::write(version_dir.join(format!("{loader_id}.json")), &raw).await?;

    // 复制父版本 natives，加载器沿用原版的本地库
    if let Some(parent_id) = profile.inherits_from.clone() {
        let parent_natives = natives_dir(cfg, &parent_id);
        let target = natives_dir(cfg, &loader_id);
        if parent_natives.exists() {
            tokio::fs::create_dir_all(&target).await?;
            let source = parent_natives.clone();
            let dest = target.clone();
            tokio::task::spawn_blocking(move || copy_dir_all(&source, &dest))
                .await??;
        }
    }

    let merged = resolve_detail(cfg, &loader_id)?;
    let mut tasks: Vec<FileTask> = Vec::new();
    for lib in &merged.detail.libraries {
        if !rules_allow(lib.rules.as_ref()) || library_is_native(lib) {
            continue;
        }
        if let Some((dl, path)) = library_artifact(lib) {
            let dest = cfg.game_root().join("libraries").join(path.replace('/', "\\"));
            if !dest.exists() {
                tasks.push(FileTask {
                    url: dl.url,
                    dest,
                    sha1: Some(dl.sha1),
                    size: dl.size,
                    label: lib.name.clone(),
                });
            }
        }
    }

    if !tasks.is_empty() {
        progress.set_message(format!("正在下载 {loader} 依赖库…"));
        download::run_tasks(
            app,
            client,
            tasks,
            cfg.concurrency as usize,
            use_mirror,
            progress.clone(),
        )
        .await?;
    }

    progress.set_message("加载器安装完成");
    progress.active.store(false, Ordering::Relaxed);
    download::emit_now(app, &progress);

    let java_major = merged.detail.required_java();
    let jar_size = merged
        .jar_id
        .as_ref()
        .and_then(|jar_id| {
            std::fs::metadata(
                cfg.game_root()
                    .join("versions")
                    .join(jar_id)
                    .join(format!("{jar_id}.jar")),
            )
            .ok()
        })
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(InstalledVersion {
        id: loader_id,
        kind: "modded".into(),
        jar_size,
        has_jar: true,
        last_modified: std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        java_major,
    })
}
