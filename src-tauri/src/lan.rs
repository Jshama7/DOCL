use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 2, 60);
const PORT: u16 = 4445;
const ROOM_TTL: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LanRoom {
    pub id: String,
    pub motd: String,
    pub host: String,
    pub port: u16,
    pub address: String,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LanView {
    pub rooms: Vec<LanRoom>,
    pub error: Option<String>,
}

/// 直连目标，负责统一 host/port 的格式。
#[derive(Debug, Clone)]
pub struct ServerTarget {
    pub host: String,
    pub port: u16,
}

impl ServerTarget {
    pub fn address(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

struct LanState {
    rooms: HashMap<String, LanRoom>,
    error: Option<String>,
}

static STATE: OnceLock<Mutex<LanState>> = OnceLock::new();

fn state() -> &'static Mutex<LanState> {
    STATE.get_or_init(|| {
        Mutex::new(LanState {
            rooms: HashMap::new(),
            error: None,
        })
    })
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn between(text: &str, start: &str, end: &str) -> Option<String> {
    let from = text.find(start)? + start.len();
    let rest = &text[from..];
    let to = rest.find(end)?;
    Some(rest[..to].to_string())
}

fn split_address(raw: &str, fallback_host: &str) -> (String, u16) {
    let value = raw.trim();
    if value.is_empty() {
        return (fallback_host.to_string(), PORT);
    }
    if let Some(rest) = value.strip_prefix('[') {
        if let Some((host, tail)) = rest.split_once(']') {
            let port = tail
                .strip_prefix(':')
                .and_then(|port| port.parse::<u16>().ok())
                .unwrap_or(PORT);
            return (host.to_string(), port);
        }
    }
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return (fallback_host.to_string(), value.parse::<u16>().unwrap_or(PORT));
    }
    match value.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) => (
            host.to_string(),
            port.parse::<u16>().unwrap_or(PORT),
        ),
        _ => (value.to_string(), PORT),
    }
}

fn parse_room(payload: &str, source: SocketAddr) -> Option<LanRoom> {
    let text = payload.trim();
    let fallback_host = source.ip().to_string();
    let motd = between(text, "[MOTD]", "[/MOTD]").unwrap_or_default();
    let raw_address = between(text, "[ADDR]", "[/ADDR]").or_else(|| between(text, "[AD]", "[/AD]"));

    let (host, port) = if let Some(raw) = raw_address {
        split_address(&raw, &fallback_host)
    } else if let Some(rest) = text.strip_prefix("[ADVERTISED]") {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Advertised {
            motd: String,
            port: u16,
        }
        let advertised: Advertised = serde_json::from_str(rest).ok()?;
        (
            fallback_host.clone(),
            if advertised.port == 0 {
                PORT
            } else {
                advertised.port
            },
        )
    } else {
        return None;
    };

    if host.trim().is_empty() || port == 0 {
        return None;
    }
    let address = format!("{host}:{port}");
    let title = motd.trim().to_string();
    Some(LanRoom {
        id: address.clone(),
        motd: if title.is_empty() {
            address.clone()
        } else {
            title
        },
        host,
        port,
        address,
        last_seen: now_secs(),
    })
}

fn prune(state: &mut LanState) {
    let now = now_secs();
    state
        .rooms
        .retain(|_, room| now.saturating_sub(room.last_seen) <= ROOM_TTL);
}

fn listen() {
    let socket = match UdpSocket::bind(("0.0.0.0", PORT)) {
        Ok(socket) => socket,
        Err(err) => {
            state().lock().unwrap().error =
                Some(format!("无法监听局域网端口 {PORT}：{err}"));
            return;
        }
    };
    if let Err(err) = socket.join_multicast_v4(&GROUP, &Ipv4Addr::UNSPECIFIED) {
        state().lock().unwrap().error = Some(format!("加入局域网组播失败：{err}"));
    }
    let _ = socket.set_read_timeout(Some(Duration::from_millis(800)));
    let mut buffer = [0u8; 4096];
    loop {
        match socket.recv_from(&mut buffer) {
            Ok((len, source)) => {
                if let Ok(text) = std::str::from_utf8(&buffer[..len]) {
                    if let Some(room) = parse_room(text, source) {
                        let mut guard = state().lock().unwrap();
                        guard.error = None;
                        guard.rooms.insert(room.address.clone(), room);
                    }
                }
            }
            Err(_) => {}
        }
        let mut guard = state().lock().unwrap();
        prune(&mut guard);
    }
}

pub fn start() {
    std::thread::spawn(listen);
}

pub fn rooms() -> LanView {
    let mut guard = state().lock().unwrap();
    prune(&mut guard);
    let mut rooms: Vec<LanRoom> = guard.rooms.values().cloned().collect();
    rooms.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    LanView {
        rooms,
        error: guard.error.clone(),
    }
}

pub fn parse_address(raw: &str) -> Result<ServerTarget> {
    let value = raw.trim();
    if value.is_empty() {
        bail!("请输入服务器地址");
    }
    if value.chars().any(|ch| ch.is_whitespace()) {
        bail!("服务器地址不能包含空格");
    }
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| anyhow::anyhow!("IPv6 地址缺少右括号"))?;
        let port = match tail.trim() {
            "" => 25565,
            other => other
                .strip_prefix(':')
                .ok_or_else(|| anyhow::anyhow!("IPv6 地址格式应为 [地址]:端口"))?
                .parse::<u16>()?,
        };
        (host.to_string(), port)
    } else if value.matches(':').count() > 1 {
        (value.to_string(), 25565)
    } else {
        match value.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() => (host.to_string(), port.parse::<u16>()?),
            _ => (value.to_string(), 25565),
        }
    };
    if host.is_empty() {
        bail!("服务器地址不能为空");
    }
    if port == 0 {
        bail!("端口必须在 1-65535 之间");
    }
    Ok(ServerTarget { host, port })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ad_with_port_only() {
        let source: SocketAddr = "192.168.1.9:50000".parse().unwrap();
        let room = parse_room("[MOTD]朋友的存档[/MOTD][AD]54321[/AD]", source).unwrap();
        assert_eq!(room.motd, "朋友的存档");
        assert_eq!(room.host, "192.168.1.9");
        assert_eq!(room.port, 54321);
        assert_eq!(room.address, "192.168.1.9:54321");
        assert_eq!(room.id, "192.168.1.9:54321");
    }

    #[test]
    fn parse_addr_with_explicit_host() {
        let source: SocketAddr = "192.168.1.9:50000".parse().unwrap();
        let room = parse_room("[MOTD]旧版广播[/MOTD][ADDR]10.0.0.2:25565[/ADDR]", source).unwrap();
        assert_eq!(room.host, "10.0.0.2");
        assert_eq!(room.port, 25565);
    }

    #[test]
    fn parse_direct_addresses() {
        let plain = parse_address("play.example.com").unwrap();
        assert_eq!(plain.host, "play.example.com");
        assert_eq!(plain.port, 25565);
        assert_eq!(plain.address(), "play.example.com:25565");

        let with_port = parse_address("192.168.1.2:25570").unwrap();
        assert_eq!(with_port.host, "192.168.1.2");
        assert_eq!(with_port.port, 25570);

        let ipv6 = parse_address("[::1]:25565").unwrap();
        assert_eq!(ipv6.host, "::1");
        assert_eq!(ipv6.address(), "[::1]:25565");

        assert!(parse_address("  ").is_err());
        assert!(parse_address("bad host").is_err());
    }
}
