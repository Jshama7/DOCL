//! 已安装 Mod 的来源识别：本地 jar 的 SHA1 查 Modrinth，CurseForge 指纹兜底，
//! 用来在版本设置里显示图标与项目链接（识别不到的模组保持原样）。

use crate::config::Config;
use crate::instance;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

const CF_BASE: &str = "https://mod.mcimirror.top/curseforge/v1";
const CF_FALLBACK: &str = "https://api.curse.tools/v1/cf";
const MR_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize)]
pub struct ModMeta {
    pub file_name: String,
    pub name: String,
    pub project_id: String,
    /// modrinth / curseforge
    pub source: String,
    pub logo_url: String,
    pub website_url: String,
    pub version_number: String,
}

/// CurseForge 的文件指纹：MurmurHash2 取正后对素数取模。
fn cf_fingerprint(data: &[u8]) -> u32 {
    const M: u32 = 0x5bd1_e995;
    let len = data.len() as u32;
    let mut h: u32 = 1 ^ len;
    let mut i = 0usize;
    while data.len() - i >= 4 {
        let mut k = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
        i += 4;
    }
    let rest = data.len() - i;
    if rest == 3 {
        h ^= (data[i + 2] as u32) << 16;
    }
    if rest >= 2 {
        h ^= (data[i + 1] as u32) << 8;
    }
    if rest >= 1 {
        h ^= data[i] as u32;
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    (h & 0x7fff_ffff) % 2_147_483_647
}

#[derive(Deserialize)]
struct MrVersion {
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    version_number: String,
}

#[derive(Deserialize)]
struct MrProject {
    #[serde(default)]
    id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    icon_url: Option<String>,
}

#[derive(Deserialize)]
struct CfLogo {
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct CfLinks {
    #[serde(default, rename = "websiteUrl")]
    website_url: String,
}

#[derive(Deserialize)]
struct CfFingerprintFile {
    #[serde(default, rename = "modId")]
    mod_id: i64,
    #[serde(default, rename = "fileName")]
    file_name: String,
}

#[derive(Deserialize)]
struct CfFingerprintItem {
    #[serde(default)]
    file: Option<CfFingerprintFile>,
}

#[derive(Deserialize)]
struct CfFingerprintResponse {
    #[serde(default)]
    data: Vec<CfFingerprintItem>,
}

#[derive(Deserialize)]
struct CfMod {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    links: Option<CfLinks>,
    #[serde(default)]
    logo: Option<CfLogo>,
}

#[derive(Deserialize)]
struct CfModsResponse {
    #[serde(default)]
    data: Vec<CfMod>,
}

async fn post_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<T> {
    let resp = client
        .post(url)
        .json(body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }
    Ok(resp.json::<T>().await?)
}

/// 扫描实例 mods 目录并反查来源；识别不到的项目直接跳过。
pub async fn lookup(
    client: &reqwest::Client,
    cfg: &Config,
    version_id: &str,
) -> Result<Vec<ModMeta>> {
    let dir = instance::game_dir(cfg, version_id).join("mods");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    // 本地文件 → (sha1, fingerprint)
    let mut local: Vec<(String, String, u32)> = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        if !lower.ends_with(".jar") && !lower.ends_with(".jar.disabled") && !lower.ends_with(".disabled")
        {
            continue;
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size == 0 || size > 200 * 1024 * 1024 {
            continue;
        }
        let Ok(bytes) = read_bytes(&path) else {
            continue;
        };
        let sha1 = format!("{:x}", Sha1::digest(&bytes));
        let fingerprint = cf_fingerprint(&bytes);
        local.push((name, sha1, fingerprint));
    }
    if local.is_empty() {
        return Ok(Vec::new());
    }

    let mut result: HashMap<String, ModMeta> = HashMap::new();

    // 1) Modrinth：sha1 批量反查
    let hashes: Vec<String> = local.iter().map(|(_, sha1, _)| sha1.clone()).collect();
    if let Ok(map) = post_json::<HashMap<String, MrVersion>>(
        client,
        &format!("{MR_BASE}/version_files"),
        &serde_json::json!({ "hashes": hashes, "algorithm": "sha1" }),
    )
    .await
    {
        let ids: Vec<String> = map
            .values()
            .map(|version| version.project_id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        if !ids.is_empty() {
            if let Ok(projects) = post_json::<Vec<MrProject>>(
                client,
                &format!("{MR_BASE}/projects"),
                &serde_json::json!({ "ids": ids }),
            )
            .await
            {
                let by_id: HashMap<String, MrProject> = projects
                    .into_iter()
                    .map(|project| (project.id.clone(), project))
                    .collect();
                for (file_name, sha1, _) in &local {
                    let Some(version) = map.get(sha1) else {
                        continue;
                    };
                    let Some(project) = by_id.get(&version.project_id) else {
                        continue;
                    };
                    let key = base_name(file_name);
                    result.insert(
                        key.clone(),
                        ModMeta {
                            file_name: key,
                            name: project.title.clone(),
                            project_id: project.id.clone(),
                            source: "modrinth".into(),
                            logo_url: project.icon_url.clone().unwrap_or_default(),
                            website_url: if project.slug.is_empty() {
                                format!("https://modrinth.com/project/{}", project.id)
                            } else {
                                format!("https://modrinth.com/mod/{}", project.slug)
                            },
                            version_number: version.version_number.clone(),
                        },
                    );
                }
            }
        }
    }

    // 2) CurseForge：剩下的用指纹查
    let pending: Vec<(String, u32)> = local
        .iter()
        .filter(|(file_name, _, _)| !result.contains_key(&base_name(file_name)))
        .map(|(file_name, _, fingerprint)| (file_name.clone(), *fingerprint))
        .collect();
    if !pending.is_empty() {
        let fingerprints: Vec<u32> = pending.iter().map(|(_, fp)| *fp).collect();
        let mut matched: Vec<(String, i64)> = Vec::new();
        for base in [CF_BASE, CF_FALLBACK] {
            let url = format!("{base}/fingerprints");
            let body = serde_json::json!({ "fingerprints": fingerprints });
            match post_json::<CfFingerprintResponse>(client, &url, &body).await {
                Ok(response) => {
                    for item in response.data {
                        let Some(file) = item.file else { continue };
                        if file.mod_id > 0 && !file.file_name.is_empty() {
                            matched.push((file.file_name, file.mod_id));
                        }
                    }
                    break;
                }
                Err(_) => continue,
            }
        }
        if !matched.is_empty() {
            let mod_ids: Vec<i64> = matched.iter().map(|(_, id)| *id).collect();
            let url = format!("{CF_BASE}/mods");
            let body = serde_json::json!({ "modIds": mod_ids });
            let mods: Option<CfModsResponse> = match post_json(client, &url, &body).await {
                Ok(value) => Some(value),
                Err(_) => post_json(
                    client,
                    &format!("{CF_FALLBACK}/mods"),
                    &body,
                )
                .await
                .ok(),
            };
            if let Some(mods) = mods {
                let by_id: HashMap<i64, CfMod> =
                    mods.data.into_iter().map(|item| (item.id, item)).collect();
                for (file_name, mod_id) in matched {
                    // 指纹返回的文件名可能带路径，按文件名对齐本地文件
                    let key = base_name(
                        &Path::new(&file_name)
                            .file_name()
                            .map(|value| value.to_string_lossy().to_string())
                            .unwrap_or(file_name),
                    );
                    let Some(project) = by_id.get(&mod_id) else {
                        continue;
                    };
                    let website = project
                        .links
                        .as_ref()
                        .map(|links| links.website_url.clone())
                        .filter(|url| !url.is_empty())
                        .unwrap_or_else(|| {
                            format!("https://www.curseforge.com/projects/{mod_id}")
                        });
                    result.insert(
                        key.clone(),
                        ModMeta {
                            file_name: key,
                            name: project.name.clone(),
                            project_id: mod_id.to_string(),
                            source: "curseforge".into(),
                            logo_url: project
                                .logo
                                .as_ref()
                                .map(|logo| logo.url.clone())
                                .unwrap_or_default(),
                            website_url: website,
                            version_number: String::new(),
                        },
                    );
                }
            }
        }
    }

    Ok(result.into_values().collect())
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// 前端按实例文件列表里的名字查询，禁用的模组名带 .disabled 后缀，统一去掉。
pub fn base_name(name: &str) -> String {
    name.trim_end_matches(".disabled").to_string()
}
