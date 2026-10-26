use crate::config::Config;
use crate::download;
use crate::install;
use crate::launch;
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LOG_LINES: usize = 400;

#[derive(Debug, Clone, Serialize)]
pub struct ServerVersionOption {
    pub id: String,
    pub game_version: String,
    pub java_major: u32,
    pub loader: String,
    /// 服务端整合包的显示名；普通版本为空
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerSnapshot {
    pub version_id: String,
    pub game_version: String,
    pub pid: u32,
    pub port: u16,
    pub lan_address: String,
    pub local_address: String,
    pub started_at: u64,
    pub online: bool,
    pub attached: bool,
    pub loader: String,
    pub dir: String,
    pub java: String,
    pub log: Vec<String>,
}

/// 启动器退出后仍保留的服务端信息，用于下次启动时接管。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedServer {
    version_id: String,
    game_version: String,
    pid: u32,
    port: u16,
    lan_address: String,
    local_address: String,
    started_at: u64,
    dir: String,
    java: String,
    #[serde(default)]
    loader: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerOptions {
    pub port: u16,
    pub motd: String,
    pub max_players: u32,
    pub view_distance: u32,
    pub online_mode: bool,
    pub white_list: bool,
    pub accept_eula: bool,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            port: 25565,
            motd: "DOCL 本地服务器".into(),
            max_players: 10,
            view_distance: 8,
            online_mode: true,
            white_list: false,
            accept_eula: false,
        }
    }
}

pub struct ServerRuntime {
    version_id: String,
    game_version: String,
    pid: u32,
    port: u16,
    lan_address: String,
    local_address: String,
    started_at: u64,
    dir: PathBuf,
    java: String,
    log: Arc<Mutex<VecDeque<String>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    online: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    attached: bool,
    loader: String,
}

impl ServerRuntime {
    fn snapshot(&self) -> ServerSnapshot {
        let log = self
            .log
            .lock()
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default();
        ServerSnapshot {
            version_id: self.version_id.clone(),
            game_version: self.game_version.clone(),
            pid: self.pid,
            port: self.port,
            lan_address: self.lan_address.clone(),
            local_address: self.local_address.clone(),
            started_at: self.started_at,
            online: self.online.load(Ordering::Relaxed),
            attached: self.attached,
            loader: self.loader.clone(),
            dir: self.dir.to_string_lossy().to_string(),
            java: self.java.clone(),
            log,
        }
    }
}

static SLOT: OnceLock<Mutex<Option<ServerRuntime>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<ServerRuntime>> {
    SLOT.get_or_init(|| Mutex::new(None))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn persisted_path() -> PathBuf {
    crate::config::data_dir().join("server.json")
}

fn save_persisted(snapshot: &ServerSnapshot, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(crate::config::data_dir())?;
    let data = PersistedServer {
        version_id: snapshot.version_id.clone(),
        game_version: snapshot.game_version.clone(),
        pid: snapshot.pid,
        port: snapshot.port,
        lan_address: snapshot.lan_address.clone(),
        local_address: snapshot.local_address.clone(),
        started_at: snapshot.started_at,
        dir: dir.to_string_lossy().to_string(),
        java: snapshot.java.clone(),
        loader: snapshot.loader.clone(),
    };
    std::fs::write(persisted_path(), serde_json::to_vec_pretty(&data)?)?;
    Ok(())
}

fn load_persisted() -> Option<PersistedServer> {
    let bytes = std::fs::read(persisted_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn clear_persisted() {
    let _ = std::fs::remove_file(persisted_path());
}

/// 判断 PID 对应的进程是否仍然存活，并确认它是 Java 进程，避免 PID 复用误判。
fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let mut command = Command::new("tasklist");
    command
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let Ok(output) = command.output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    text.contains("java")
}

fn port_open(port: u16) -> bool {
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&address, Duration::from_millis(400)).is_ok()
}

/// 找不到运行中的实例时，从 server.json 恢复一个只能查看/停止的接管状态。
fn attach_persisted() -> Option<ServerSnapshot> {
    let saved = load_persisted()?;
    if !process_alive(saved.pid) {
        clear_persisted();
        return None;
    }
    let log = Arc::new(Mutex::new(VecDeque::from([format!(
        "已接管启动器上次退出前运行的服务端（PID {}）。控制台输出与命令输入不可用，可停止或复制地址。",
        saved.pid
    )])));
    let online = Arc::new(AtomicBool::new(port_open(saved.port)));
    let runtime = ServerRuntime {
        version_id: saved.version_id,
        game_version: saved.game_version,
        pid: saved.pid,
        port: saved.port,
        lan_address: saved.lan_address,
        local_address: saved.local_address,
        started_at: saved.started_at,
        dir: PathBuf::from(saved.dir),
        java: saved.java,
        log,
        stdin: Arc::new(Mutex::new(None)),
        online,
        alive: Arc::new(AtomicBool::new(true)),
        attached: true,
        loader: saved.loader,
    };
    let snapshot = runtime.snapshot();
    *slot().lock().unwrap() = Some(runtime);
    Some(snapshot)
}

fn safe_id(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn server_dir(cfg: &Config, version_id: &str) -> PathBuf {
    cfg.game_root()
        .join("docl-servers")
        .join(safe_id(version_id))
}

/// 服务端整合包实例：把实例目录里的模组、配置等复制到服务端目录，
/// 已有文件不覆盖（保留玩家改过的配置）。
fn sync_instance_payload(cfg: &Config, version_id: &str, dir: &Path) -> Result<()> {
    if crate::instance::get(cfg, version_id).category != "modpack" {
        return Ok(());
    }
    let src = cfg.game_root().join("versions").join(safe_id(version_id));
    if !src.is_dir() {
        return Ok(());
    }
    // 客户端专属目录与版本文件不复制到服务端目录
    const SKIP_DIRS: &[&str] = &[
        "versions",
        "saves",
        "logs",
        "crash-reports",
        "screenshots",
        "natives",
        "shaderpacks",
        "resourcepacks",
    ];
    fn walk(from: &Path, to: &Path, skip: &[&str], top: bool) -> Result<()> {
        for entry in std::fs::read_dir(from)?.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let target = to.join(&name);
            if path.is_dir() {
                if skip.iter().any(|item| item.eq_ignore_ascii_case(&name)) {
                    continue;
                }
                std::fs::create_dir_all(&target)?;
                walk(&path, &target, skip, false)?;
                continue;
            }
            // 顶层 <id>.json / <id>.jar 是启动器用的版本文件
            if top && (name.to_lowercase().ends_with(".json") || name.to_lowercase().ends_with(".jar")) {
                continue;
            }
            if target.exists() {
                continue;
            }
            if let Err(e) = std::fs::copy(&path, &target) {
                eprintln!("复制服务端内容失败：{} {e}", path.display());
            }
        }
        Ok(())
    }
    walk(&src, dir, SKIP_DIRS, true)
}

fn pick_port(preferred: u16) -> u16 {
    let start = if preferred == 0 { 25565 } else { preferred };
    for offset in 0..20u16 {
        let port = start.saturating_add(offset);
        if port == 0 {
            continue;
        }
        if TcpListener::bind(("0.0.0.0", port)).is_ok() {
            return port;
        }
    }
    start
}

fn lan_ip() -> String {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(address) = socket.local_addr() {
                if !address.ip().is_loopback() {
                    return address.ip().to_string();
                }
            }
        }
    }
    "127.0.0.1".into()
}

fn clean_motd(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

/// server.properties 按 ISO-8859-1 读取，非 ASCII 字符需要转成 \\uXXXX。
fn escape_property(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ':' => out.push_str("\\:"),
            '=' => out.push_str("\\="),
            ch if ch.is_ascii() => out.push(ch),
            ch => {
                let mut buffer = [0u16; 2];
                for unit in ch.encode_utf16(&mut buffer).iter() {
                    out.push_str(&format!("\\u{unit:04X}"));
                }
            }
        }
    }
    out
}

fn update_properties(dir: &Path, options: &ServerOptions, port: u16) -> Result<()> {
    let path = dir.join("server.properties");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let wanted: Vec<(&str, String)> = vec![
        ("server-port", port.to_string()),
        ("motd", {
            let motd = clean_motd(&options.motd);
            escape_property(if motd.is_empty() {
                "DOCL 本地服务器"
            } else {
                &motd
            })
        }),
        ("max-players", options.max_players.clamp(1, 200).to_string()),
        ("view-distance", options.view_distance.clamp(3, 32).to_string()),
        ("online-mode", bool_text(options.online_mode).to_string()),
        ("white-list", bool_text(options.white_list).to_string()),
        ("enforce-whitelist", bool_text(options.white_list).to_string()),
    ];

    let mut lines: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for line in existing.lines() {
        let key = line
            .split_once('=')
            .map(|(key, _)| key.trim())
            .unwrap_or_default();
        if let Some((name, value)) = wanted.iter().find(|(name, _)| *name == key) {
            lines.push(format!("{name}={value}"));
            seen.insert(*name);
        } else {
            lines.push(line.to_string());
        }
    }
    for (name, value) in &wanted {
        if !seen.contains(name) {
            lines.push(format!("{name}={value}"));
        }
    }
    std::fs::write(&path, format!("{}\n", lines.join("\n")))?;
    Ok(())
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn verify_sha1(bytes: &[u8], expected: &str) -> bool {
    if expected.trim().is_empty() {
        return true;
    }
    let actual = format!("{:x}", Sha1::digest(bytes));
    actual.eq_ignore_ascii_case(expected.trim())
}

/// 从合并后的版本信息里识别加载器类型与版本。
struct LoaderInfo {
    loader: String,
    version: String,
}

fn loader_info(merged: &install::MergedDetail) -> Option<LoaderInfo> {
    for library in &merged.detail.libraries {
        let parts: Vec<&str> = library.name.split(':').collect();
        if parts.len() < 3 {
            continue;
        }
        let (group, artifact, version) = (parts[0], parts[1], parts[2]);
        match (group, artifact) {
            ("net.fabricmc", "fabric-loader") => {
                return Some(LoaderInfo {
                    loader: "fabric".into(),
                    version: version.to_string(),
                })
            }
            ("org.quiltmc", "quilt-loader") => {
                return Some(LoaderInfo {
                    loader: "quilt".into(),
                    version: version.to_string(),
                })
            }
            ("net.minecraftforge", "forge") => {
                return Some(LoaderInfo {
                    loader: "forge".into(),
                    version: version
                        .split_once('-')
                        .map(|(_, value)| value.to_string())
                        .unwrap_or_else(|| version.to_string()),
                })
            }
            ("net.neoforged", "neoforge") => {
                return Some(LoaderInfo {
                    loader: "neoforge".into(),
                    version: version.to_string(),
                })
            }
            ("net.neoforged", "forge") => {
                return Some(LoaderInfo {
                    loader: "neoforge".into(),
                    version: version
                        .split_once('-')
                        .map(|(_, value)| value.to_string())
                        .unwrap_or_else(|| version.to_string()),
                })
            }
            _ => {}
        }
    }
    None
}

/// 可以开服的已安装版本：纯原版 + 已支持的加载器。
pub fn versions(cfg: &Config) -> Vec<ServerVersionOption> {
    let mut result = Vec::new();
    for item in install::list_installed(cfg) {
        let Ok(merged) = install::resolve_detail(cfg, &item.id) else {
            continue;
        };
        let game_version = merged
            .chain
            .last()
            .cloned()
            .unwrap_or_else(|| item.id.clone());
        let info = loader_info(&merged);
        let loader = if merged.chain.len() == 1 && info.is_none() {
            if merged
                .detail
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.server.as_ref())
                .is_none()
            {
                continue;
            }
            "vanilla".to_string()
        } else if let Some(info) = info {
            if !matches!(info.loader.as_str(), "fabric" | "forge" | "neoforge") {
                continue;
            }
            info.loader
        } else {
            continue;
        };
        result.push(ServerVersionOption {
            id: item.id,
            game_version,
            java_major: merged.detail.required_java(),
            loader,
            name: String::new(),
        });
    }
    // 已导入的服务端整合包：直接从解压目录启动
    for pack in load_packs() {
        if !Path::new(&pack.dir).is_dir() {
            continue;
        }
        result.push(ServerVersionOption {
            id: pack.id,
            game_version: pack.game_version,
            java_major: pack.java_major,
            loader: pack.loader,
            name: pack.name,
        });
    }
    result
}

pub fn status() -> Option<ServerSnapshot> {
    let mut guard = slot().lock().unwrap();
    if let Some(runtime) = guard.as_ref() {
        if runtime.attached {
            if !process_alive(runtime.pid) {
                *guard = None;
                clear_persisted();
                return None;
            }
            let mut snapshot = runtime.snapshot();
            snapshot.online = port_open(runtime.port);
            return Some(snapshot);
        }
        if !runtime.alive.load(Ordering::Relaxed) {
            *guard = None;
            return None;
        }
        return Some(runtime.snapshot());
    }
    drop(guard);
    attach_persisted()
}

enum LaunchMode {
    Jar(PathBuf),
    Args(Vec<PathBuf>),
}

fn installer_cache() -> PathBuf {
    crate::config::data_dir().join("installers")
}

async fn ensure_installer(
    client: &reqwest::Client,
    url: &str,
    file_name: &str,
) -> Result<PathBuf> {
    let dir = installer_cache();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(file_name);
    if std::fs::metadata(&path)
        .map(|meta| meta.len() > 10_000)
        .unwrap_or(false)
    {
        return Ok(path);
    }
    let bytes = download::get_bytes(client, url, false)
        .await
        .with_context(|| format!("下载安装器失败：{file_name}"))?;
    if bytes.len() < 10_000 {
        bail!("安装器下载不完整：{file_name}");
    }
    std::fs::write(&path, bytes)?;
    Ok(path)
}

async fn run_installer(
    java: &str,
    installer: &Path,
    args: &[String],
    dir: &Path,
    log: &Arc<Mutex<VecDeque<String>>>,
) -> Result<()> {
    let mut command = tokio::process::Command::new(java);
    command
        .arg("-jar")
        .arg(installer)
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let child = command.spawn().context("启动加载器安装器失败")?;
    let output = tokio::time::timeout(Duration::from_secs(900), child.wait_with_output())
        .await
        .map_err(|_| anyhow!("加载器安装器运行超时"))??;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let tail: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .rev()
        .take(6)
        .collect();
    if !tail.is_empty() {
        if let Ok(mut lines) = log.lock() {
            for line in tail.into_iter().rev() {
                lines.push_back(format!(
                    "[安装器] {}",
                    line.trim().chars().take(200).collect::<String>()
                ));
            }
        }
    }
    if !output.status.success() {
        bail!(
            "加载器安装器退出码：{}",
            output.status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

async fn fabric_installer_version(client: &reqwest::Client) -> String {
    let url = "https://maven.fabricmc.net/net/fabricmc/fabric-installer/maven-metadata.xml";
    if let Ok(bytes) = download::get_bytes(client, url, false).await {
        let text = String::from_utf8_lossy(&bytes);
        if let Some(rest) = text.split("<release>").nth(1) {
            if let Some(version) = rest.split("</release>").next() {
                let version = version.trim();
                if !version.is_empty() {
                    return version.to_string();
                }
            }
        }
    }
    "1.1.2".into()
}

async fn prepare_vanilla(
    client: &reqwest::Client,
    merged: &install::MergedDetail,
    dir: &Path,
) -> Result<LaunchMode> {
    let server_ref = merged
        .detail
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.server.clone())
        .ok_or_else(|| anyhow!("该版本没有官方服务端"))?;
    let jar_path = dir.join("server.jar");
    let need_download = match std::fs::metadata(&jar_path) {
        Ok(meta) => meta.len() == 0 || (server_ref.size > 0 && meta.len() != server_ref.size),
        Err(_) => true,
    };
    if need_download {
        if server_ref.url.trim().is_empty() {
            bail!("服务端下载地址为空");
        }
        let bytes = download::get_bytes(client, &server_ref.url, true)
            .await
            .context("下载服务端失败")?;
        if !verify_sha1(&bytes, &server_ref.sha1) {
            bail!("服务端文件校验失败，请重试");
        }
        std::fs::write(&jar_path, &bytes)?;
    }
    Ok(LaunchMode::Jar(jar_path))
}

async fn prepare_fabric(
    client: &reqwest::Client,
    java: &str,
    dir: &Path,
    game_version: &str,
    loader_version: &str,
    log: &Arc<Mutex<VecDeque<String>>>,
) -> Result<LaunchMode> {
    let launch = dir.join("fabric-server-launch.jar");
    if !launch.exists() {
        let version = fabric_installer_version(client).await;
        let file_name = format!("fabric-installer-{version}.jar");
        let url = format!(
            "https://maven.fabricmc.net/net/fabricmc/fabric-installer/{version}/{file_name}"
        );
        let installer = ensure_installer(client, &url, &file_name).await?;
        let args = vec![
            "server".to_string(),
            "-mcversion".to_string(),
            game_version.to_string(),
            "-loader".to_string(),
            loader_version.to_string(),
            "-downloadMinecraft".to_string(),
            "-dir".to_string(),
            dir.to_string_lossy().to_string(),
        ];
        run_installer(java, &installer, &args, dir, log).await?;
    }
    if !launch.exists() {
        bail!("Fabric 服务端生成失败（缺少 fabric-server-launch.jar）");
    }
    Ok(LaunchMode::Jar(launch))
}

/// 旧版 Forge 安装器不会生成 win_args.txt，而是留下 universal / server jar。
fn find_forge_jar(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.ends_with(".jar") && name.contains("forge") && (name.contains("universal") || name.contains("server")) {
            return Some(path);
        }
    }
    None
}

fn find_win_args(root: &Path) -> Option<PathBuf> {
    fn walk(path: &Path, depth: usize) -> Option<PathBuf> {
        if depth > 10 {
            return None;
        }
        let entries = std::fs::read_dir(path).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = walk(&path, depth + 1) {
                    return Some(found);
                }
            } else if entry.file_name().to_string_lossy() == "win_args.txt" {
                return Some(path);
            }
        }
        None
    }
    walk(&root.join("libraries"), 0)
}

async fn prepare_forge_like(
    client: &reqwest::Client,
    cfg: &Config,
    java: &str,
    dir: &Path,
    game_version: &str,
    info: &LoaderInfo,
    log: &Arc<Mutex<VecDeque<String>>>,
) -> Result<LaunchMode> {
    if find_win_args(dir).is_none() && find_forge_jar(dir).is_none() {
        let (url, file_name) = if info.loader == "neoforge" {
            let short = if game_version == "1.20.1" {
                info.version.trim_start_matches("1.20.1-").to_string()
            } else {
                info.version.clone()
            };
            let (_, url, file) = install::neoforge_installer_for(game_version, &short);
            (url, file)
        } else {
            let (_, url, file) = install::forge_installer_for(game_version, &info.version);
            (url, file)
        };
        let installer = ensure_installer(client, &url, &file_name).await?;
        let mut args = vec![
            "--installServer".to_string(),
            dir.to_string_lossy().to_string(),
        ];
        if cfg.use_mirror() {
            args.push("--mirror".into());
            args.push("https://bmclapi2.bangbang93.com/maven".into());
        }
        run_installer(java, &installer, &args, dir, log).await?;
    }

    let Some(win_args) = find_win_args(dir) else {
        if let Some(jar) = find_forge_jar(dir) {
            return Ok(LaunchMode::Jar(jar));
        }
        bail!("安装器没有生成可用的启动参数，服务端安装失败");
    };
    let mut files = Vec::new();
    let user_args = dir.join("user_jvm_args.txt");
    if user_args.exists() {
        files.push(user_args);
    }
    files.push(win_args);
    Ok(LaunchMode::Args(files))
}

pub async fn start(
    client: &reqwest::Client,
    cfg: &Config,
    version_id: &str,
    options: &ServerOptions,
) -> Result<ServerSnapshot> {
    if !options.accept_eula {
        bail!("需要先阅读并同意 Minecraft EULA");
    }
    if status().is_some() {
        bail!("已经有一个本地服务端在运行");
    }

    let merged = install::resolve_detail(cfg, version_id)?;
    let game_version = merged
        .chain
        .last()
        .cloned()
        .unwrap_or_else(|| version_id.to_string());
    let required = merged.detail.required_java();
    let info = loader_info(&merged);
    let loader = if merged.chain.len() == 1 && info.is_none() {
        "vanilla".to_string()
    } else {
        info.as_ref()
            .map(|item| item.loader.clone())
            .ok_or_else(|| anyhow!("无法识别该版本的加载器"))?
    };
    if loader == "quilt" {
        bail!("Quilt 服务端暂未支持，请使用 Fabric / Forge / NeoForge");
    }

    let dir = server_dir(cfg, version_id);
    std::fs::create_dir_all(&dir)?;
    // 服务端整合包：先把实例内容同步过来，再准备启动文件
    sync_instance_payload(cfg, version_id, &dir)?;
    let log: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    {
        let mut lines = log.lock().unwrap();
        lines.push_back(format!("版本：{version_id}（{loader}）"));
        lines.push_back(format!("游戏版本：{game_version}"));
    }

    let java = launch::find_java(cfg, required)?;
    let mode = match loader.as_str() {
        "vanilla" => prepare_vanilla(client, &merged, &dir).await?,
        "fabric" => {
            let info = info.as_ref().ok_or_else(|| anyhow!("无法识别 Fabric 加载器版本"))?;
            prepare_fabric(
                client,
                &java.path,
                &dir,
                &game_version,
                &info.version,
                &log,
            )
            .await?
        }
        "forge" | "neoforge" => {
            let info = info
                .as_ref()
                .ok_or_else(|| anyhow!("无法识别 {loader} 加载器版本"))?;
            prepare_forge_like(
                client,
                cfg,
                &java.path,
                &dir,
                &game_version,
                info,
                &log,
            )
            .await?
        }
        other => bail!("{other} 服务端暂未支持"),
    };

    spawn_server(
        cfg,
        &dir,
        &java,
        &mode,
        options,
        version_id,
        &game_version,
        &loader,
        log,
    )
}

/// 真正拉起服务端进程：写 eula / server.properties、启动 Java、接管日志与状态。
#[allow(clippy::too_many_arguments)]
fn spawn_server(
    cfg: &Config,
    dir: &Path,
    java: &crate::java::JavaInfo,
    mode: &LaunchMode,
    options: &ServerOptions,
    version_id: &str,
    game_version: &str,
    loader: &str,
    log: Arc<Mutex<VecDeque<String>>>,
) -> Result<ServerSnapshot> {
    std::fs::write(dir.join("eula.txt"), "eula=true\n")?;
    let port = pick_port(options.port);
    update_properties(dir, options, port)?;

    let memory = launch::effective_memory_mb(cfg);
    let mut command = Command::new(&java.path);
    command
        .current_dir(dir)
        .arg(format!("-Xmx{memory}M"))
        .arg(format!("-Xms{}M", (memory / 2).clamp(256, 2048)))
        .arg("-XX:+UseG1GC")
        .arg("-Dfile.encoding=UTF-8")
        .arg("-Dstdout.encoding=UTF-8")
        .arg("-Dstderr.encoding=UTF-8");
    for extra in cfg.java_args.split_whitespace() {
        if !extra.trim().is_empty() {
            command.arg(extra);
        }
    }
    match &mode {
        LaunchMode::Jar(path) => {
            let relative = path.strip_prefix(dir).unwrap_or(path);
            command.arg("-jar").arg(relative).arg("nogui");
        }
        LaunchMode::Args(files) => {
            for file in files {
                let relative = file.strip_prefix(dir).unwrap_or(file);
                command.arg(format!("@{}", relative.display()));
            }
            command.arg("nogui");
        }
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command.spawn().context("启动服务端进程失败")?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    let online = Arc::new(AtomicBool::new(false));
    let alive = Arc::new(AtomicBool::new(true));
    let stdin_slot = Arc::new(Mutex::new(stdin));

    {
        let mut lines = log.lock().unwrap();
        lines.push_back(format!("使用 Java：{}", java.path));
        lines.push_back(format!("端口：{port}"));
    }

    spawn_reader(stdout, log.clone(), online.clone(), false);
    spawn_reader(stderr, log.clone(), online.clone(), true);

    let alive_for_wait = alive.clone();
    let log_for_wait = log.clone();
    let online_for_wait = online.clone();
    std::thread::spawn(move || {
        let status = child.wait();
        alive_for_wait.store(false, Ordering::Relaxed);
        online_for_wait.store(false, Ordering::Relaxed);
        if let Ok(mut lines) = log_for_wait.lock() {
            lines.push_back(format!(
                "服务端已退出：{}",
                status
                    .ok()
                    .and_then(|status| status.code())
                    .map(|code| format!("退出码 {code}"))
                    .unwrap_or_else(|| "进程结束".into())
            ));
        }
        if load_persisted()
            .map(|saved| saved.pid == pid)
            .unwrap_or(false)
        {
            clear_persisted();
        }
        let _ = slot().lock().map(|mut guard| guard.take());
    });

    let runtime = ServerRuntime {
        version_id: version_id.to_string(),
        game_version: game_version.to_string(),
        pid,
        port,
        lan_address: format!("{}:{port}", lan_ip()),
        local_address: format!("127.0.0.1:{port}"),
        started_at: now_secs(),
        dir: dir.to_path_buf(),
        java: java.path.clone(),
        log,
        stdin: stdin_slot,
        online,
        alive,
        attached: false,
        loader: loader.to_string(),
    };
    let snapshot = runtime.snapshot();
    save_persisted(&snapshot, dir)?;
    *slot().lock().unwrap() = Some(runtime);
    Ok(snapshot)
}

fn spawn_reader(
    stream: Option<impl std::io::Read + Send + 'static>,
    log: Arc<Mutex<VecDeque<String>>>,
    online: Arc<AtomicBool>,
    is_error: bool,
) {
    let Some(stream) = stream else {
        return;
    };
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            let text = if is_error {
                format!("[stderr] {line}")
            } else {
                line
            };
            if text.contains("Done (") && text.contains("For help") {
                online.store(true, Ordering::Relaxed);
            }
            if let Ok(mut lines) = log.lock() {
                lines.push_back(text);
                while lines.len() > MAX_LOG_LINES {
                    lines.pop_front();
                }
            }
        }
    });
}

pub fn send(command: &str) -> Result<()> {
    let line = command.replace(['\r', '\n'], " ");
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    let guard = slot().lock().unwrap();
    let runtime = guard
        .as_ref()
        .ok_or_else(|| anyhow!("服务端未运行"))?;
    let mut stdin = runtime.stdin.lock().unwrap();
    let stdin = stdin.as_mut().ok_or_else(|| {
        if runtime.attached {
            anyhow!("启动器重启后无法向服务端发送命令，可直接停止服务端")
        } else {
            anyhow!("服务端控制台不可用")
        }
    })?;
    writeln!(stdin, "{line}")?;
    stdin.flush()?;
    Ok(())
}

enum StopAction {
    Sent,
    Attached(u32),
    Kill(u32),
    Cleanup,
    Persisted,
}

pub fn stop() -> Result<()> {
    let action = {
        let guard = slot().lock().unwrap();
        match guard.as_ref() {
            None => StopAction::Persisted,
            Some(runtime) if runtime.attached => StopAction::Attached(runtime.pid),
            Some(runtime) if !runtime.alive.load(Ordering::Relaxed) => StopAction::Cleanup,
            Some(runtime) => {
                let mut stdin = runtime.stdin.lock().unwrap();
                match stdin.as_mut() {
                    Some(stdin) => {
                        if writeln!(stdin, "stop").is_ok() && stdin.flush().is_ok() {
                            StopAction::Sent
                        } else {
                            StopAction::Kill(runtime.pid)
                        }
                    }
                    None => StopAction::Kill(runtime.pid),
                }
            }
        }
    };

    match action {
        StopAction::Sent => Ok(()),
        StopAction::Attached(pid) | StopAction::Kill(pid) => {
            *slot().lock().unwrap() = None;
            clear_persisted();
            launch::kill(pid)
        }
        StopAction::Cleanup => {
            *slot().lock().unwrap() = None;
            clear_persisted();
            Ok(())
        }
        StopAction::Persisted => {
            if let Some(saved) = load_persisted() {
                clear_persisted();
                launch::kill(saved.pid)?;
            }
            Ok(())
        }
    }
}

/* ---------- 服务端整合包：解压到独立目录后直接启动 ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPack {
    /// 形如 pack:<slug>，与已安装版本 ID 区分
    pub id: String,
    pub name: String,
    pub dir: String,
    pub game_version: String,
    pub java_major: u32,
    pub loader: String,
    pub imported_at: u64,
}

fn packs_path() -> PathBuf {
    crate::config::data_dir().join("server-packs.json")
}

pub fn load_packs() -> Vec<ServerPack> {
    std::fs::read(packs_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_packs(packs: &[ServerPack]) -> Result<()> {
    std::fs::create_dir_all(crate::config::data_dir())?;
    std::fs::write(packs_path(), serde_json::to_vec_pretty(packs)?)?;
    Ok(())
}

fn pack_slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch.is_whitespace() || ch == '.' {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        format!("pack-{}", now_secs())
    } else {
        trimmed.chars().take(40).collect()
    }
}

/// 从文件名里找形如 1.12.2 / 1.21 的 Minecraft 版本。
fn find_version(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 2 < bytes.len() {
        if bytes[i] == b'1' && bytes[i + 1] == b'.' && (i == 0 || !bytes[i - 1].is_ascii_digit()) {
            let rest = &text[i..];
            let mut end = 0usize;
            let mut dots = 0usize;
            for (offset, ch) in rest.char_indices() {
                if ch.is_ascii_digit() {
                    end = offset + ch.len_utf8();
                } else if ch == '.' && dots < 2 {
                    dots += 1;
                    end = offset + 1;
                } else {
                    break;
                }
            }
            let candidate = rest[..end].trim_end_matches('.');
            if candidate.len() >= 3 {
                return Some(candidate.to_string());
            }
        }
        i += 1;
    }
    None
}

fn java_major_for(game_version: &str) -> u32 {
    let mut parts = game_version.split('.');
    let major = parts.next().and_then(|value| value.parse::<u32>().ok()).unwrap_or(1);
    if major >= 21 {
        return 21;
    }
    if major != 1 {
        return 17;
    }
    let minor = parts.next().and_then(|value| value.parse::<u32>().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|value| value.parse::<u32>().ok()).unwrap_or(0);
    if minor >= 21 || (minor == 20 && patch >= 5) {
        21
    } else if minor >= 18 {
        17
    } else if minor == 17 {
        16
    } else {
        8
    }
}

fn detect_pack_launch(dir: &Path) -> Option<LaunchMode> {
    // 1) Forge / NeoForge 1.17+ 的 @win_args.txt 方式
    if let Some(win_args) = find_win_args(dir) {
        let mut files = Vec::new();
        let user_args = dir.join("user_jvm_args.txt");
        if user_args.exists() {
            files.push(user_args);
        }
        files.push(win_args);
        return Some(LaunchMode::Args(files));
    }
    // 2) 根目录的各个 jar，按常见优先级挑一个
    let mut jars: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !name.ends_with(".jar") || name.contains("installer") || name.starts_with("minecraft_server")
            {
                return None;
            }
            Some((name, path))
        })
        .collect();
    let prefer = [
        "fabric-server",
        "forge-",
        "neoforge-",
        "paper-",
        "purpur-",
        "spigot-",
        "server.jar",
    ];
    for key in prefer {
        if let Some((_, path)) = jars.iter().find(|(name, _)| name.starts_with(key)) {
            return Some(LaunchMode::Jar(path.clone()));
        }
    }
    jars.sort_by(|a, b| a.0.cmp(&b.0));
    jars.into_iter().next().map(|(_, path)| LaunchMode::Jar(path))
}

fn detect_pack_loader(dir: &Path) -> String {
    let has = |needle: &str| -> bool {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries.flatten().any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(needle)
                })
            })
            .unwrap_or(false)
    };
    if has("fabric-server") {
        return "fabric".into();
    }
    if dir.join("libraries").join("net").join("neoforged").is_dir() {
        return "neoforge".into();
    }
    if dir.join("libraries").join("net").join("minecraftforge").is_dir() || has("forge-") {
        return "forge".into();
    }
    if dir.join("libraries").join("net").join("fabricmc").is_dir() {
        return "fabric".into();
    }
    if has("paper-") || has("purpur-") || has("spigot-") {
        return "paper".into();
    }
    "vanilla".into()
}

fn guess_pack_game_version(dir: &Path) -> String {
    // 根目录文件名（forge-1.12.2-14.23.5.2860.jar、paper-1.20.1-196.jar）
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(value) = find_version(&entry.file_name().to_string_lossy()) {
                return value;
            }
        }
    }
    // libraries 下的加载器目录（libraries/net/minecraftforge/forge/1.12.2-14.23.5.2860）
    for group in ["minecraftforge/forge", "neoforged/neoforge"] {
        let path = dir.join("libraries").join(group);
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Some(value) = find_version(&entry.file_name().to_string_lossy()) {
                    return value;
                }
            }
        }
    }
    String::new()
}

fn extract_zip_into(archive: &str, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive).with_context(|| format!("无法打开压缩包：{archive}"))?;
    let mut zip = zip::ZipArchive::new(file).context("压缩包损坏或格式不支持")?;
    std::fs::create_dir_all(dest)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let target = dest.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// 导入服务端整合包：解压到 docl-servers/<slug>，记录启动所需的元信息。
pub fn import_pack(cfg: &Config, archive: &str, name: &str, slug: &str) -> Result<ServerPack> {
    let stem = Path::new(archive)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "server-pack".into());
    let display = if name.trim().is_empty() {
        stem.clone()
    } else {
        name.trim().to_string()
    };
    let hint = if slug.trim().is_empty() { display.clone() } else { slug.trim().to_string() };
    let id = format!("pack:{}", pack_slug(&hint));
    let dir = server_dir(cfg, &id.trim_start_matches("pack:"));

    // 重复导入时覆盖旧目录，保证与服务端包版本一致
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("无法清理旧目录：{}", dir.display()))?;
    }
    extract_zip_into(archive, &dir)?;

    // 有些包外面多套一层目录，进入该目录
    let mut root = dir.clone();
    if detect_pack_launch(&root).is_none() {
        if let Ok(entries) = std::fs::read_dir(&root) {
            let children: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_dir()
                        && path
                            .file_name()
                            .map(|name| {
                                let name = name.to_string_lossy();
                                name != "libraries" && name != "mods"
                            })
                            .unwrap_or(false)
                })
                .collect();
            if children.len() == 1 && detect_pack_launch(&children[0]).is_some() {
                root = children[0].clone();
            }
        }
    }

    if detect_pack_launch(&root).is_none() {
        bail!("没有找到服务端启动文件（server.jar / fabric-server-launch.jar / Forge win_args.txt）");
    }

    let game_version = guess_pack_game_version(&root);
    let loader = detect_pack_loader(&root);
    let java_major = if game_version.is_empty() {
        17
    } else {
        java_major_for(&game_version)
    };

    let pack = ServerPack {
        id,
        name: display,
        dir: root.to_string_lossy().to_string(),
        game_version,
        java_major,
        loader,
        imported_at: now_secs(),
    };
    let mut packs = load_packs();
    packs.retain(|item| item.id != pack.id);
    packs.push(pack.clone());
    save_packs(&packs)?;
    Ok(pack)
}

pub fn remove_pack(cfg: &Config, id: &str) -> Result<()> {
    let mut packs = load_packs();
    if let Some(pack) = packs.iter().find(|item| item.id == id).cloned() {
        if let Ok(root) = std::fs::canonicalize(&pack.dir) {
            // 只允许删除 docl-servers 内的目录
            if let Ok(base) = std::fs::canonicalize(cfg.game_root().join("docl-servers")) {
                if root.starts_with(base) {
                    let _ = std::fs::remove_dir_all(&root);
                }
            }
        }
    }
    packs.retain(|item| item.id != id);
    save_packs(&packs)?;
    Ok(())
}

pub async fn start_pack(
    cfg: &Config,
    id: &str,
    options: &ServerOptions,
) -> Result<ServerSnapshot> {
    if !options.accept_eula {
        bail!("需要先阅读并同意 Minecraft EULA");
    }
    if status().is_some() {
        bail!("已经有一个本地服务端在运行");
    }
    let pack = load_packs()
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| anyhow!("服务端整合包不存在，请重新导入"))?;
    let dir = PathBuf::from(&pack.dir);
    if !dir.is_dir() {
        bail!("服务端目录不存在：{}", pack.dir);
    }
    let mode = detect_pack_launch(&dir)
        .ok_or_else(|| anyhow!("没有找到可用的服务端启动文件"))?;
    let java = launch::find_java(cfg, pack.java_major)?;

    let log: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    {
        let mut lines = log.lock().unwrap();
        lines.push_back(format!("服务端整合包：{}", pack.name));
        if !pack.game_version.is_empty() {
            lines.push_back(format!("游戏版本：{}", pack.game_version));
        }
        lines.push_back(format!("加载器：{}", pack.loader));
    }
    spawn_server(
        cfg,
        &dir,
        &java,
        &mode,
        options,
        id,
        &pack.game_version,
        &pack.loader,
        log,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_properties_and_escape_motd() {
        let dir = std::env::temp_dir().join(format!("docl-server-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("server.properties"),
            "level-name=world\ndifficulty=easy\nmotd=old\n",
        )
        .unwrap();

        let options = ServerOptions {
            port: 25565,
            motd: "我的世界".into(),
            max_players: 20,
            view_distance: 10,
            online_mode: false,
            white_list: true,
            accept_eula: true,
        };
        update_properties(&dir, &options, 25566).unwrap();
        let content = std::fs::read_to_string(dir.join("server.properties")).unwrap();

        assert!(content.contains("level-name=world"));
        assert!(content.contains("difficulty=easy"));
        assert!(content.contains("server-port=25566"));
        assert!(content.contains("motd=\\u6211\\u7684\\u4E16\\u754C"));
        assert!(!content.contains("motd=old"));
        assert_eq!(content.matches("motd=").count(), 1);
        assert!(content.contains("online-mode=false"));
        assert!(content.contains("white-list=true"));
        assert!(content.contains("max-players=20"));
        assert!(content.contains("view-distance=10"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ports_are_picked_in_range() {
        let port = pick_port(0);
        assert!(port >= 25565);
    }

    #[test]
    fn detect_pack_launch_and_versions() {
        let dir = std::env::temp_dir().join(format!("docl-pack-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("forge-1.12.2-14.23.5.2860.jar"), b"x").unwrap();

        assert_eq!(
            find_version("forge-1.12.2-14.23.5.2860.jar").as_deref(),
            Some("1.12.2")
        );
        assert_eq!(find_version("paper-1.20.1-196.jar").as_deref(), Some("1.20.1"));
        assert_eq!(
            find_version("minecraft_server.1.21.jar").as_deref(),
            Some("1.21")
        );
        assert_eq!(java_major_for("1.12.2"), 8);
        assert_eq!(java_major_for("1.16.5"), 8);
        assert_eq!(java_major_for("1.17.1"), 16);
        assert_eq!(java_major_for("1.20.4"), 17);
        assert_eq!(java_major_for("1.20.5"), 21);
        assert_eq!(java_major_for("1.21.1"), 21);

        match detect_pack_launch(&dir) {
            Some(LaunchMode::Jar(path)) => {
                assert!(path.ends_with("forge-1.12.2-14.23.5.2860.jar"))
            }
            _ => panic!("未识别到 forge 服务端 jar"),
        }

        std::fs::write(dir.join("fabric-server-launch.jar"), b"x").unwrap();
        match detect_pack_launch(&dir) {
            Some(LaunchMode::Jar(path)) => {
                assert!(path.to_string_lossy().contains("fabric-server-launch"))
            }
            _ => panic!("fabric 服务端 jar 未优先"),
        }

        let args_dir = dir.join("libraries/net/minecraftforge/forge/1.20.1-47.2.0");
        std::fs::create_dir_all(&args_dir).unwrap();
        std::fs::write(args_dir.join("win_args.txt"), b"").unwrap();
        assert!(matches!(
            detect_pack_launch(&dir),
            Some(LaunchMode::Args(_))
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore]
    fn list_real_server_versions() {
        let cfg = crate::config::load();
        let list = versions(&cfg);
        println!("{list:?}");
        assert!(!list.is_empty(), "本机没有可开服的原版版本");
    }
}
