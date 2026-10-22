use crate::auth::{self, Account};
use crate::config;
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::path::PathBuf;

const SKIN_SIZE: u32 = 64;
const MAX_SKIN_BYTES: usize = 2 * 1024 * 1024;

pub fn skins_dir() -> PathBuf {
    config::data_dir().join("skins")
}

fn safe_id(account_id: &str) -> String {
    account_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn local_skin_path(account_id: &str) -> PathBuf {
    skins_dir().join(format!("{}.png", safe_id(account_id)))
}

pub fn to_data_url(bytes: &[u8], mime: &str) -> String {
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

fn decode_base64(raw: &str) -> Result<Vec<u8>> {
    let value = raw
        .split_once(',')
        .map(|(_, rest)| rest)
        .unwrap_or(raw)
        .trim();
    if value.len() > MAX_SKIN_BYTES * 2 {
        bail!("皮肤文件过大（上限 2 MB）");
    }
    STANDARD.decode(value).context("皮肤 base64 解码失败")
}

/// 校验 PNG 并统一为 64×64。64×32 旧皮肤会补成现代布局。
pub fn normalize_png(bytes: Vec<u8>) -> Result<Vec<u8>> {
    if bytes.len() > MAX_SKIN_BYTES {
        bail!("皮肤文件过大（上限 2 MB）");
    }
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        bail!("不是有效的 PNG 图片");
    }
    let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    let mut reader = decoder.read_info().context("解码 PNG 失败")?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    if width != SKIN_SIZE || (height != SKIN_SIZE && height != 32) {
        bail!("皮肤尺寸必须是 64×64 或 64×32，当前为 {width}×{height}");
    }
    if height == SKIN_SIZE {
        return Ok(bytes);
    }

    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buffer)?;
    buffer.truncate(frame.buffer_size());
    let source = buffer;

    let mut out = vec![0u8; (SKIN_SIZE * SKIN_SIZE * 4) as usize];
    let copy_rect = |out: &mut Vec<u8>, sx: u32, sy: u32, w: u32, h: u32, dx: u32, dy: u32| {
        for y in 0..h {
            for x in 0..w {
                let src = ((sy + y) * width + sx + x) as usize * 4;
                let dst = ((dy + y) * SKIN_SIZE + dx + x) as usize * 4;
                if src + 3 < source.len() && dst + 3 < out.len() {
                    out[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
                }
            }
        }
    };
    // 64×32 的头部/身体/右臂/右腿平移到现代布局
    copy_rect(&mut out, 0, 0, 32, 16, 0, 0);
    copy_rect(&mut out, 16, 16, 24, 16, 16, 16);
    copy_rect(&mut out, 40, 16, 16, 16, 40, 16);
    copy_rect(&mut out, 0, 16, 16, 16, 0, 16);
    // 左腿/左臂由右腿/右臂水平镜像生成
    for y in 0..16u32 {
        for x in 0..16u32 {
            let src = ((16 + y) * width + x) as usize * 4;
            let dst = ((48 + y) * SKIN_SIZE + (16 + (15 - x))) as usize * 4;
            if src + 3 < source.len() && dst + 3 < out.len() {
                out[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
            }
            let src_arm = ((16 + y) * width + 40 + x) as usize * 4;
            let dst_arm = ((48 + y) * SKIN_SIZE + (32 + (15 - x))) as usize * 4;
            if src_arm + 3 < source.len() && dst_arm + 3 < out.len() {
                out[dst_arm..dst_arm + 4].copy_from_slice(&source[src_arm..src_arm + 4]);
            }
        }
    }

    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, SKIN_SIZE, SKIN_SIZE);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&out)?;
    }
    Ok(encoded)
}

pub fn decode_and_normalize(raw: &str) -> Result<Vec<u8>> {
    normalize_png(decode_base64(raw)?)
}

pub fn save_local(account_id: &str, raw_base64: &str) -> Result<PathBuf> {
    let bytes = decode_and_normalize(raw_base64)?;
    std::fs::create_dir_all(skins_dir())?;
    let path = local_skin_path(account_id);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

pub fn read_local_bytes(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

pub fn current_bytes(account: &Account) -> Option<Vec<u8>> {
    account
        .skin_path
        .as_ref()
        .and_then(|path| read_local_bytes(path))
}

#[derive(Deserialize)]
struct ProfileProperty {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Deserialize)]
struct ProfileResponse {
    #[serde(default)]
    properties: Vec<ProfileProperty>,
}

#[derive(Deserialize)]
struct TexturePayload {
    textures: TextureMap,
}

#[derive(Deserialize)]
struct TextureMap {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Deserialize)]
struct SkinTexture {
    url: String,
    #[serde(default)]
    metadata: Option<SkinMetadata>,
}

#[derive(Deserialize)]
struct SkinMetadata {
    #[serde(default)]
    model: String,
}

pub async fn official_skin(
    client: &reqwest::Client,
    uuid: &str,
) -> Result<(Vec<u8>, String)> {
    let profile: ProfileResponse = client
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("读取 Mojang 皮肤档案失败")?;

    let property = profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .ok_or_else(|| anyhow!("档案里没有皮肤信息"))?;
    let decoded = STANDARD
        .decode(property.value.as_bytes())
        .context("皮肤属性解码失败")?;
    let payload: TexturePayload =
        serde_json::from_slice(&decoded).context("皮肤属性解析失败")?;
    let skin = payload
        .textures
        .skin
        .ok_or_else(|| anyhow!("该账号没有设置皮肤"))?;
    let model = skin
        .metadata
        .as_ref()
        .map(|m| m.model.clone())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "classic".into());
    let bytes = client
        .get(&skin.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();
    Ok((bytes, model))
}

pub async fn fallback_skin(
    client: &reqwest::Client,
    uuid: &str,
) -> Result<Vec<u8>> {
    let bytes = client
        .get(format!("https://mc-heads.net/skin/{uuid}"))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();
    Ok(bytes)
}

pub async fn load_skin(
    client: &reqwest::Client,
    account: &Account,
) -> (String, String, String) {
    if let Some(bytes) = current_bytes(account) {
        let model = if account.skin_model.is_empty() {
            "classic".to_string()
        } else {
            account.skin_model.clone()
        };
        return (to_data_url(&bytes, "image/png"), model, "local".into());
    }
    if account.kind == "microsoft" {
        if let Ok((bytes, model)) = official_skin(client, &account.uuid).await {
            return (to_data_url(&bytes, "image/png"), model, "official".into());
        }
        if let Ok(bytes) = fallback_skin(client, &account.uuid).await {
            return (
                to_data_url(&bytes, "image/png"),
                "classic".into(),
                "official".into(),
            );
        }
    }
    (String::new(), account.skin_model.clone(), "default".into())
}

pub async fn upload_skin(
    client: &reqwest::Client,
    account: &Account,
    raw_base64: &str,
    model: &str,
) -> Result<()> {
    if account.kind != "microsoft" {
        bail!("只有微软账号可以上传官方皮肤");
    }
    let bytes = decode_and_normalize(raw_base64)?;
    let refreshed = auth::refresh(client, account)
        .await
        .context("刷新微软登录凭据失败")?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("skin.png")
        .mime_str("image/png")?;
    let form = reqwest::multipart::Form::new()
        .text("variant", model.to_string())
        .part("file", part);
    let response = client
        .put("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&refreshed.access_token)
        .multipart(form)
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let text = response.text().await.unwrap_or_default();
    let message = match status.as_u16() {
        401 => "登录凭据已失效，请重新登录微软账号".to_string(),
        403 => "Mojang 拒绝上传（账号可能未拥有 Java 版或令牌权限不足）".to_string(),
        429 => "上传过于频繁，请稍后再试".to_string(),
        _ => format!("上传失败（{status}）：{text}"),
    };
    bail!(message)
}

pub fn export_bytes(account: &Account, target: &str) -> Result<()> {
    let bytes = current_bytes(account).ok_or_else(|| anyhow!("该账号还没有本地皮肤"))?;
    if let Some(parent) = std::path::Path::new(target).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, bytes)?;
    Ok(())
}
