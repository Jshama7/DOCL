use crate::config;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BASE: &str = "https://launchercontent.mojang.com";
const NEWS_URL: &str = "https://launchercontent.mojang.com/v2/news.json";
const PATCH_URL: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";
const CACHE_TTL: u64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub tag: String,
    pub date: String,
    pub summary: String,
    pub image_url: Option<String>,
    pub url: Option<String>,
    pub source: String,
    pub version: Option<String>,
    pub kind: Option<String>,
    pub detail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewsFeed {
    pub items: Vec<NewsItem>,
    pub updated_at: u64,
    pub stale: bool,
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    fetched_at: u64,
    #[serde(default)]
    items: Vec<NewsItem>,
}

#[derive(Deserialize)]
struct ImageRef {
    #[serde(default)]
    url: String,
}

#[derive(Deserialize)]
struct OfficialFeed {
    #[serde(default)]
    entries: Vec<OfficialEntry>,
}

#[derive(Deserialize)]
struct OfficialEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    date: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(rename = "newsType", default)]
    news_type: Vec<String>,
    #[serde(rename = "readMoreLink", default)]
    read_more_link: Option<String>,
    #[serde(rename = "playPageImage", default)]
    play_page_image: Option<ImageRef>,
    #[serde(rename = "newsPageImage", default)]
    news_page_image: Option<ImageRef>,
}

#[derive(Deserialize)]
struct PatchFeed {
    #[serde(default)]
    entries: Vec<PatchEntry>,
}

#[derive(Deserialize)]
struct PatchEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    date: String,
    #[serde(rename = "shortText", default)]
    short_text: String,
    #[serde(default)]
    image: Option<ImageRef>,
    #[serde(rename = "contentPath", default)]
    content_path: String,
}

#[derive(Deserialize)]
struct PatchDetail {
    #[serde(default)]
    body: String,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn cache_path() -> PathBuf {
    config::data_dir().join("news_cache.json")
}

fn read_cache() -> Option<CacheFile> {
    std::fs::read(cache_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn write_cache(items: &[NewsItem]) {
    let _ = std::fs::create_dir_all(config::data_dir());
    let payload = serde_json::json!({
        "fetched_at": now_secs(),
        "items": items,
    });
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        let _ = std::fs::write(cache_path(), bytes);
    }
}

fn image_url(image: Option<ImageRef>) -> Option<String> {
    image.and_then(|image| {
        let url = image.url.trim();
        if url.is_empty() {
            None
        } else if url.starts_with("http://") || url.starts_with("https://") {
            Some(url.to_string())
        } else if url.starts_with('/') {
            Some(format!("{BASE}{url}"))
        } else {
            Some(format!("{BASE}/{url}"))
        }
    })
}

fn truncate_text(input: &str, max: usize) -> String {
    let cleaned = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = cleaned.chars();
    let mut out: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

fn is_java_news(entry: &OfficialEntry) -> bool {
    entry
        .news_type
        .iter()
        .any(|item| item.eq_ignore_ascii_case("java"))
        || entry.category.to_lowercase().contains("java")
}

async fn fetch_official(client: &reqwest::Client) -> Result<Vec<NewsItem>> {
    let feed: OfficialFeed = client
        .get(NEWS_URL)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .context("请求官方新闻失败")?
        .error_for_status()?
        .json()
        .await
        .context("解析官方新闻失败")?;

    let mut items: Vec<NewsItem> = feed
        .entries
        .into_iter()
        .filter(is_java_news)
        .map(|entry| NewsItem {
            id: entry.id,
            title: entry.title.trim().to_string(),
            category: "Java Edition".into(),
            tag: entry
                .tag
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .unwrap_or_else(|| "NEWS".into()),
            date: entry.date.trim().to_string(),
            summary: truncate_text(entry.text.as_deref().unwrap_or_default(), 160),
            image_url: image_url(entry.play_page_image.or(entry.news_page_image)),
            url: entry
                .read_more_link
                .filter(|link| !link.trim().is_empty()),
            source: "official".into(),
            version: None,
            kind: None,
            detail_path: None,
        })
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items.truncate(12);
    Ok(items)
}

fn patch_tag(kind: &str) -> String {
    match kind {
        "snapshot" => "快照".into(),
        "release" => "正式版".into(),
        other if !other.is_empty() => other.to_string(),
        _ => "更新".into(),
    }
}

async fn fetch_patch_notes(client: &reqwest::Client) -> Result<Vec<NewsItem>> {
    let feed: PatchFeed = client
        .get(PATCH_URL)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .context("请求版本更新日志失败")?
        .error_for_status()?
        .json()
        .await
        .context("解析版本更新日志失败")?;

    let mut items: Vec<NewsItem> = feed
        .entries
        .into_iter()
        .map(|entry| NewsItem {
            id: entry.id,
            title: entry.title.trim().to_string(),
            category: "Java 版本".into(),
            tag: patch_tag(&entry.kind),
            date: entry.date.chars().take(10).collect(),
            summary: truncate_text(&entry.short_text, 160),
            image_url: image_url(entry.image),
            url: None,
            source: "patch".into(),
            version: if entry.version.trim().is_empty() {
                None
            } else {
                Some(entry.version)
            },
            kind: if entry.kind.trim().is_empty() {
                None
            } else {
                Some(entry.kind)
            },
            detail_path: if entry.content_path.trim().is_empty() {
                None
            } else {
                Some(entry.content_path)
            },
        })
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items.truncate(8);
    Ok(items)
}

/// 获取新闻聚合，优先命中缓存；失败时回退到旧缓存。
pub async fn feed(client: &reqwest::Client, force: bool) -> NewsFeed {
    let now = now_secs();
    if !force {
        if let Some(cache) = read_cache() {
            if now.saturating_sub(cache.fetched_at) < CACHE_TTL {
                return NewsFeed {
                    items: cache.items,
                    updated_at: cache.fetched_at,
                    stale: false,
                    errors: Vec::new(),
                };
            }
        }
    }

    let (official, patch) = futures::join!(fetch_official(client), fetch_patch_notes(client));
    let mut items = Vec::new();
    let mut errors = Vec::new();
    match official {
        Ok(mut list) => items.append(&mut list),
        Err(err) => errors.push(format!("官方新闻：{err}")),
    }
    match patch {
        Ok(mut list) => items.append(&mut list),
        Err(err) => errors.push(format!("版本日志：{err}")),
    }

    if items.is_empty() {
        if let Some(cache) = read_cache() {
            return NewsFeed {
                items: cache.items,
                updated_at: cache.fetched_at,
                stale: true,
                errors,
            };
        }
        return NewsFeed {
            items: Vec::new(),
            updated_at: now,
            stale: true,
            errors,
        };
    }

    items.sort_by(|a, b| b.date.cmp(&a.date));
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(format!("{}::{}", item.source, item.id)));
    write_cache(&items);

    NewsFeed {
        items,
        updated_at: now,
        stale: false,
        errors,
    }
}

/// 把更新日志 HTML 转成可读纯文本，避免前端直接渲染 HTML。
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();
    for tag in ["</h1>", "</h2>", "</h3>", "</p>", "</li>", "</ul>", "</ol>"] {
        text = text.replace(tag, "\n");
    }
    for tag in ["<br>", "<br/>", "<br />"] {
        text = text.replace(tag, "\n");
    }
    text = text.replace("<li>", "\n• ");

    let mut stripped = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => stripped.push(ch),
            _ => {}
        }
    }

    let decoded = stripped
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    let mut lines: Vec<String> = Vec::new();
    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if lines.last().map(|last| !last.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
        } else {
            lines.push(trimmed.to_string());
        }
    }
    while lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines.join("\n")
}

/// 拉取单条版本更新日志正文。
pub async fn article(client: &reqwest::Client, path: &str) -> Result<String> {
    let relative = path.trim();
    if !relative.starts_with("javaPatchNotes/") || relative.contains("..") {
        bail!("无效的更新日志路径");
    }
    let detail: PatchDetail = client
        .get(format!("{BASE}/v2/{relative}"))
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .context("请求更新日志失败")?
        .error_for_status()?
        .json()
        .await
        .context("解析更新日志失败")?;
    let text = html_to_text(&detail.body);
    if text.trim().is_empty() {
        bail!("更新日志正文为空");
    }
    Ok(text)
}
