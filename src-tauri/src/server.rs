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

    std::fs::write(dir.join("eula.txt"), "eula=true\n")?;
    let port = pick_port(options.port);
    update_properties(&dir, options, port)?;

    let memory = launch::effective_memory_mb(cfg);
    let mut command = Command::new(&java.path);
    command
        .current_dir(&dir)
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
            let relative = path.strip_prefix(&dir).unwrap_or(path);
            command.arg("-jar").arg(relative).arg("nogui");
        }
        LaunchMode::Args(files) => {
            for file in files {
                let relative = file.strip_prefix(&dir).unwrap_or(file);
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
        game_version,
        pid,
        port,
        lan_address: format!("{}:{port}", lan_ip()),
        local_address: format!("127.0.0.1:{port}"),
        started_at: now_secs(),
        dir: dir.clone(),
        java: java.path,
        log,
        stdin: stdin_slot,
        online,
        alive,
        attached: false,
        loader,
    };
    let snapshot = runtime.snapshot();
    save_persisted(&snapshot, &dir)?;
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
    #[ignore]
    fn list_real_server_versions() {
        let cfg = crate::config::load();
        let list = versions(&cfg);
        println!("{list:?}");
        assert!(!list.is_empty(), "本机没有可开服的原版版本");
    }
}
