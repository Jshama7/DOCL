//! MC 百科中文名：按项目英文名检索百科，取回模组中文名。
//!
//! 说明：
//! - 百科搜索页是服务端渲染的 HTML，这里做轻量解析，不引入额外依赖。
//! - mcmod 的 robots.txt 禁止抓取 `/s`，因此做了严格节流（请求间隔 ≥ 800ms）、
//!   本地缓存 30 天，并且支持在设置里关闭。
//! - 解析不到（或结果没有中文名）时返回空，前端继续显示源站原名。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SEARCH_URL: &str = "https://search.mcmod.cn/s";
const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";
/// 命中缓存 30 天，未命中 3 天（新模组可能后收录）
const TTL_HIT: u64 = 30 * 24 * 3600;
const TTL_MISS: u64 = 3 * 24 * 3600;
/// 单次批量最多查 8 个，避免一次点开列表打出太多请求
const BATCH_LIMIT: usize = 8;
/// 两次请求之间的最小间隔（百科对短时连续请求会返回限流页）
const REQUEST_GAP_MS: u64 = 1500;

#[derive(Debug, Clone, Deserialize)]
pub struct NameQuery {
    /// 前端索引键（source-id）
    pub key: String,
    /// 项目英文名
    pub title: String,
    /// 项目 slug，作为第二匹配依据
    #[serde(default)]
    pub slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChineseName {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    name: String,
    url: String,
    at: u64,
    hit: bool,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn cache_path() -> PathBuf {
    crate::config::data_dir().join("mcmod-cache.json")
}

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(load_cache()))
}

fn load_cache() -> HashMap<String, CacheEntry> {
    std::fs::read(cache_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_cache(map: &HashMap<String, CacheEntry>) {
    let _ = std::fs::create_dir_all(crate::config::data_dir());
    if let Ok(bytes) = serde_json::to_vec(map) {
        let _ = std::fs::write(cache_path(), bytes);
    }
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn has_cjk(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch as u32,
            0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
        )
    })
}

fn decode_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn strip_tags(raw: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if in_tag => {}
            _ => out.push(ch),
        }
    }
    decode_entities(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// 把 `[JEI] JEI物品管理器 (Just Enough Items)` 拆成（中文名, 英文名）。
fn split_entry(text: &str) -> (String, String) {
    let mut text = text.trim();
    // 去掉开头的 [简称]
    if text.starts_with('[') {
        if let Some(index) = text.find(']') {
            text = text[index + 1..].trim();
        }
    }
    // 英文名在最后一对括号里（半角或全角）
    let open = text.rfind('(').or_else(|| text.rfind('（'));
    let close = text.rfind(')').or_else(|| text.rfind('）'));
    if let (Some(open), Some(close)) = (open, close) {
        if close > open {
            let english = text[open + 1..close].trim().to_string();
            let chinese = text[..open].trim().to_string();
            // 只有「中文名 + 括号英文名」的组合才算有效条目；
            // 像 “Just Enough Items (Legacy)” 这种括号属于标题本身，按无中文名处理。
            if has_cjk(&chinese) && !has_cjk(&english) && !chinese.is_empty() {
                return (chinese, english);
            }
        }
    }
    (String::new(), text.to_string())
}

struct Candidate {
    id: String,
    chinese: String,
    english: String,
}

fn collect_candidates(html: &str) -> Vec<Candidate> {
    let mut result: Vec<Candidate> = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = html[cursor..].find("href=\"") {
        let href_start = cursor + found + "href=\"".len();
        let Some(quote) = html[href_start..].find('"') else {
            break;
        };
        let href = &html[href_start..href_start + quote];
        cursor = href_start + quote + 1;
        // 只关心百科条目页：https://www.mcmod.cn/class/459.html
        let Some(class_pos) = href.find("mcmod.cn/class/") else {
            continue;
        };
        let rest = &href[class_pos + "mcmod.cn/class/".len()..];
        let Some(dot) = rest.find(".html") else {
            continue;
        };
        let id = &rest[..dot];
        if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        // 取 <a ...>文本</a>
        let Some(close_tag) = html[cursor..].find('>') else {
            break;
        };
        let text_start = cursor + close_tag + 1;
        let Some(end) = html[text_start..].find("</a>") else {
            continue;
        };
        let raw = &html[text_start..text_start + end];
        let text = strip_tags(raw);
        if text.is_empty() {
            continue;
        }
        let (chinese, english) = split_entry(&text);
        if let Some(existing) = result.iter_mut().find(|item| item.id == id) {
            // 同一 id 会出现两次（结果条目 + 页脚地址），保留带中文名的那次
            if existing.chinese.is_empty() && !chinese.is_empty() {
                existing.chinese = chinese;
                existing.english = english;
            }
            continue;
        }
        result.push(Candidate {
            id: id.to_string(),
            chinese,
            english,
        });
    }
    result
}

fn pick_best(html: &str, title: &str, slug: &str) -> Option<ChineseName> {
    let targets: Vec<String> = [normalize(title), normalize(slug)]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect();
    if targets.is_empty() {
        return None;
    }
    let mut best: Option<(i32, ChineseName)> = None;
    for candidate in collect_candidates(html) {
        if candidate.chinese.is_empty() {
            continue;
        }
        let english = normalize(&candidate.english);
        if english.is_empty() {
            continue;
        }
        let score = if targets.iter().any(|target| *target == english) {
            100
        } else if english.len() >= 4
            && targets.iter().any(|target| {
                target.len() >= 4 && (english.starts_with(target) || target.starts_with(&english))
            })
        {
            60
        } else {
            0
        };
        if score == 0 {
            continue;
        }
        if best.as_ref().map(|(value, _)| score > *value).unwrap_or(true) {
            best = Some((
                score,
                ChineseName {
                    name: candidate.chinese,
                    url: format!("https://www.mcmod.cn/class/{}.html", candidate.id),
                },
            ));
        }
    }
    best.map(|(_, value)| value)
}

/// 保证两次请求之间至少间隔 REQUEST_GAP_MS，避免给百科造成压力。
async fn polite_gap() {
    static LAST: OnceLock<Mutex<u64>> = OnceLock::new();
    let slot = LAST.get_or_init(|| Mutex::new(0));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let wait = {
        let mut guard = slot.lock().unwrap();
        let start = now.max(*guard);
        *guard = start + REQUEST_GAP_MS;
        start.saturating_sub(now)
    };
    if wait > 0 {
        tokio::time::sleep(Duration::from_millis(wait)).await;
    }
}

/// 项目名里常带 " (JEI)"、" [Forge]" 之类的后缀，检索时去掉能提高命中率。
fn search_keyword(title: &str) -> String {
    let mut text = title.trim();
    for (open, close) in [('(', ')'), ('[', ']'), ('（', '）'), ('【', '】')] {
        if let (Some(start), Some(end)) = (text.rfind(open), text.rfind(close)) {
            if end > start && start > 0 {
                text = text[..start].trim();
            }
        }
    }
    if text.is_empty() {
        title.trim().to_string()
    } else {
        text.to_string()
    }
}

async fn search_name(
    client: &reqwest::Client,
    title: &str,
    slug: &str,
) -> Result<Option<ChineseName>> {
    polite_gap().await;
    let keyword = search_keyword(title);
    let resp = client
        .get(SEARCH_URL)
        .query(&[("key", keyword.as_str())])
        .header(reqwest::header::USER_AGENT, UA)
        .header(
            reqwest::header::ACCEPT_LANGUAGE,
            "zh-CN,zh;q=0.9,en;q=0.8",
        )
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .timeout(Duration::from_secs(20))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("MC 百科返回 HTTP {}", resp.status());
    }
    let html = resp.text().await?;
    // 访问过快时百科会返回一段提示页（含“频繁”），这类结果不能当作“未收录”缓存。
    // 注意正常页面也有 meta robots，不能按 robot 判断。
    if html.contains("频繁") || html.contains("请稍后再试") {
        anyhow::bail!("MC 百科限流");
    }
    Ok(pick_best(&html, title, slug))
}

/// 批量取中文名：命中缓存直接返回，未缓存的按顺序节流查询。
pub async fn batch(
    client: &reqwest::Client,
    queries: Vec<NameQuery>,
) -> HashMap<String, ChineseName> {
    let mut result: HashMap<String, ChineseName> = HashMap::new();
    let mut pending: Vec<(String, String, String)> = Vec::new();
    let now = now_secs();
    {
        let guard = cache().lock().unwrap();
        for query in &queries {
            let key = normalize(&query.title);
            if key.is_empty() {
                continue;
            }
            if let Some(entry) = guard.get(&key) {
                let ttl = if entry.hit { TTL_HIT } else { TTL_MISS };
                if now.saturating_sub(entry.at) < ttl {
                    if entry.hit && !entry.name.is_empty() {
                        result.insert(
                            query.key.clone(),
                            ChineseName {
                                name: entry.name.clone(),
                                url: entry.url.clone(),
                            },
                        );
                    }
                    continue;
                }
            }
            pending.push((
                query.key.clone(),
                query.title.clone(),
                query.slug.clone(),
            ));
        }
    }

    let mut updated = false;
    for (key, title, slug) in pending.into_iter().take(BATCH_LIMIT) {
        let cache_key = normalize(&title);
        if cache_key.is_empty() {
            continue;
        }
        let found = match search_name(client, &title, &slug).await {
            Ok(value) => value,
            // 限流时直接停止本批，剩下的留给下次（不写缓存）
            Err(_) => break,
        };
        let entry = match &found {
            Some(value) => CacheEntry {
                name: value.name.clone(),
                url: value.url.clone(),
                at: now_secs(),
                hit: true,
            },
            None => CacheEntry {
                name: String::new(),
                url: String::new(),
                at: now_secs(),
                hit: false,
            },
        };
        cache().lock().unwrap().insert(cache_key, entry);
        updated = true;
        if let Some(value) = found {
            result.insert(key, value);
        }
    }
    if updated {
        let snapshot = cache().lock().unwrap().clone();
        save_cache(&snapshot);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_chinese_and_english_names() {
        assert_eq!(
            split_entry("[JEI] JEI物品管理器 ( Just Enough Items )"),
            ("JEI物品管理器".to_string(), "Just Enough Items".to_string())
        );
        assert_eq!(
            split_entry("钠 ( Sodium )"),
            ("钠".to_string(), "Sodium".to_string())
        );
        let (chinese, english) = split_entry("Just Enough Items (Legacy)");
        assert!(chinese.is_empty());
        assert_eq!(english, "Just Enough Items (Legacy)");
    }

    #[test]
    fn picks_entry_whose_english_matches() {
        let html = r#"
        <div class="result-item"><a target="_blank" href="https://www.mcmod.cn/class/27363.html">Just Enough Items (Legacy)</a></div>
        <div class="result-item"><a target="_blank" href="https://www.mcmod.cn/class/459.html">[JEI] JEI物品管理器 (<em>Just</em><em>Enough</em><em>Items</em>)</a></div>
        "#;
        let picked = pick_best(html, "Just Enough Items (JEI)", "jei").expect("应命中 JEI");
        assert_eq!(picked.name, "JEI物品管理器");
        assert_eq!(picked.url, "https://www.mcmod.cn/class/459.html");
    }

    #[test]
    fn ignores_entries_without_chinese_name() {
        let html = r#"<a href="https://www.mcmod.cn/class/1.html">Just Enough Items (Legacy)</a>"#;
        assert!(pick_best(html, "Just Enough Items (Legacy)", "").is_none());
    }

    /// 联网实测：cargo test --lib mcmod::tests::live_lookup -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_lookup() {
        let client = crate::http::client();
        let cases = [
            ("Just Enough Items (JEI)", "jei"),
            ("Sodium", "sodium"),
            ("Create", "create"),
            ("Applied Energistics 2", "ae2"),
            ("Botania", "botania"),
            ("Iris Shaders", "iris"),
            ("Xaero's Minimap", "xaeros-minimap"),
            ("FerriteCore", "ferritecore"),
        ];
        let mut hit = 0;
        for (title, slug) in cases {
            match search_name(&client, title, slug).await {
                Ok(Some(value)) => {
                    hit += 1;
                    println!("{title} => {} ({})", value.name, value.url);
                }
                Ok(None) => println!("{title} => 未命中"),
                Err(err) => println!("{title} => 请求失败：{err}"),
            }
        }
        println!("命中 {hit}/{}", cases.len());
        assert!(hit >= 5, "命中率过低：{hit}/{}", cases.len());
    }

    #[tokio::test]
    #[ignore]
    async fn debug_html() {
        let client = crate::http::client();
        let resp = client
            .get(SEARCH_URL)
            .query(&[("key", "Just Enough Items")])
            .header(reqwest::header::USER_AGENT, UA)
            .header(
                reqwest::header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
            .unwrap();
        println!("status={}", resp.status());
        let html = resp.text().await.unwrap();
        println!("len={}", html.len());
        println!("head={}", html.chars().take(400).collect::<String>());
        let candidates = collect_candidates(&html);
        println!("candidates={}", candidates.len());
        for item in candidates.iter().take(6) {
            println!("id={} cn=[{}] en=[{}]", item.id, item.chinese, item.english);
        }
    }
}
