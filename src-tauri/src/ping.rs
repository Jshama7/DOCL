use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration, Instant};

use crate::lan::ServerTarget;

#[derive(Debug, Clone, Serialize)]
pub struct ServerPingResult {
    pub motd: String,
    pub version_name: String,
    pub protocol: i64,
    pub players_online: u32,
    pub players_max: u32,
    pub players_sample: Vec<String>,
    pub latency_ms: u64,
    pub favicon: Option<String>,
}

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn read_varint_bytes(data: &[u8], pos: &mut usize) -> Result<i32> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    loop {
        let byte = *data.get(*pos).ok_or_else(|| anyhow!("数据包不完整"))?;
        *pos += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            bail!("VarInt 过长");
        }
    }
    Ok(result as i32)
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).await?;
        result |= ((byte[0] & 0x7f) as u32) << shift;
        if byte[0] & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            bail!("VarInt 过长");
        }
    }
    Ok(result as i32)
}

fn packet(id: i32, fields: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, id);
    body.extend_from_slice(fields);
    let mut out = Vec::with_capacity(body.len() + 5);
    write_varint(&mut out, body.len() as i32);
    out.extend_from_slice(&body);
    out
}

pub async fn ping(target: &ServerTarget, timeout_ms: u64) -> Result<ServerPingResult> {
    let limit = Duration::from_millis(timeout_ms.max(1500));
    timeout(limit, ping_inner(target))
        .await
        .map_err(|_| anyhow!("连接超时（{} 毫秒无响应）", timeout_ms))?
}

async fn ping_inner(target: &ServerTarget) -> Result<ServerPingResult> {
    let connect_start = Instant::now();
    let mut stream = TcpStream::connect((target.host.as_str(), target.port))
        .await
        .context("无法连接服务器")?;

    // Handshake：协议 -1，next state = 1（status）
    let mut fields = Vec::new();
    write_varint(&mut fields, -1);
    write_varint(&mut fields, target.host.as_bytes().len() as i32);
    fields.extend_from_slice(target.host.as_bytes());
    fields.extend_from_slice(&target.port.to_be_bytes());
    write_varint(&mut fields, 1);
    stream.write_all(&packet(0x00, &fields)).await?;
    // Status Request
    stream.write_all(&packet(0x00, &[])).await?;

    // Status Response
    let packet_len = read_varint(&mut stream).await?;
    if packet_len <= 0 || packet_len > 1_000_000 {
        bail!("服务器响应异常");
    }
    let mut data = vec![0u8; packet_len as usize];
    stream.read_exact(&mut data).await?;
    let mut pos = 0usize;
    if read_varint_bytes(&data, &mut pos)? != 0x00 {
        bail!("服务器返回了意外的数据包");
    }
    let json_len = read_varint_bytes(&data, &mut pos)? as usize;
    let json = data
        .get(pos..pos + json_len)
        .ok_or_else(|| anyhow!("服务器响应不完整"))?;
    let value: serde_json::Value = serde_json::from_slice(json).context("解析服务器信息失败")?;

    let mut latency = connect_start.elapsed().as_millis() as u64;

    // Ping / Pong 测延迟，失败则沿用建连耗时
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0);
    let ping_start = Instant::now();
    let pong: Result<()> = async {
        stream.write_all(&packet(0x01, &millis.to_be_bytes())).await?;
        let len = read_varint(&mut stream).await?;
        if len <= 0 || len > 1024 {
            bail!("Pong 数据异常");
        }
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).await?;
        Ok(())
    }
    .await;
    if pong.is_ok() {
        latency = ping_start.elapsed().as_millis() as u64;
    }

    let description = value.get("description").cloned().unwrap_or_default();
    let players = value.get("players").cloned().unwrap_or_default();
    let version = value.get("version").cloned().unwrap_or_default();
    let sample = players
        .get("sample")
        .and_then(|item| item.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|entry| entry.get("name").and_then(|v| v.as_str()))
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(ServerPingResult {
        motd: chat_text(&description),
        version_name: version
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        protocol: version.get("protocol").and_then(|v| v.as_i64()).unwrap_or(0),
        players_online: players
            .get("online")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        players_max: players.get("max").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        players_sample: sample,
        latency_ms: latency,
        favicon: value
            .get("favicon")
            .and_then(|v| v.as_str())
            .map(|text| text.to_string()),
    })
}

/// 把 MOTD（字符串或聊天组件）转成纯文本，去掉 § 格式码并折叠空白。
fn chat_text(value: &serde_json::Value) -> String {
    let raw = match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(items) => items.iter().map(chat_text).collect::<String>(),
        serde_json::Value::Object(map) => {
            let mut out = String::new();
            if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                out.push_str(text);
            }
            if let Some(extra) = map.get("extra").and_then(|v| v.as_array()) {
                for item in extra {
                    out.push_str(&chat_text(item));
                }
            }
            out
        }
        _ => String::new(),
    };
    strip_codes(&raw)
}

fn strip_codes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '§' {
            chars.next();
            continue;
        }
        out.push(ch);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_encode() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 0);
        assert_eq!(buf, vec![0x00]);

        let mut buf = Vec::new();
        write_varint(&mut buf, 127);
        assert_eq!(buf, vec![0x7f]);

        let mut buf = Vec::new();
        write_varint(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);

        let mut buf = Vec::new();
        write_varint(&mut buf, 25565);
        assert_eq!(buf, vec![0xdd, 0xc7, 0x01]);

        let mut buf = Vec::new();
        write_varint(&mut buf, -1);
        assert_eq!(buf, vec![0xff, 0xff, 0xff, 0xff, 0x0f]);
    }

    #[test]
    fn varint_round_trip() {
        for value in [0, 1, 127, 128, 25565, 2_147_483_647, -1, -2147483648] {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            let mut pos = 0usize;
            assert_eq!(read_varint_bytes(&buf, &mut pos).unwrap(), value);
            assert_eq!(pos, buf.len());
        }
    }

    #[test]
    fn varint_rejects_truncated() {
        let data = [0x80u8];
        let mut pos = 0usize;
        assert!(read_varint_bytes(&data, &mut pos).is_err());
    }

    #[test]
    fn motd_plain_string() {
        let value = serde_json::json!("§a绿色标题 §l加粗\n第二行");
        assert_eq!(chat_text(&value), "绿色标题 加粗 第二行");
    }

    #[test]
    fn motd_chat_component() {
        let value = serde_json::json!({
            "text": "§b欢迎 ",
            "extra": [
                { "text": "来到" },
                { "text": "§c服务器", "bold": true }
            ]
        });
        assert_eq!(chat_text(&value), "欢迎 来到服务器");
    }

    #[test]
    fn motd_array_form() {
        let value = serde_json::json!(["", { "text": "你好" }, { "text": "世界" }]);
        assert_eq!(chat_text(&value), "你好世界");
    }

    #[test]
    fn strip_codes_collapses_whitespace() {
        assert_eq!(strip_codes("  多  空格\t和\n换行  "), "多 空格 和 换行");
    }

    #[test]
    fn packet_wraps_length() {
        let out = packet(0x00, &[]);
        assert_eq!(out, vec![0x01, 0x00]);
    }
}
