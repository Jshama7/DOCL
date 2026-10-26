//! 项目中文文案：借助 MCIM 的翻译接口补充中文简介。
//!
//! 说明：PCL 的「少数模组有中文名」来自它内置的 MC 百科离线数据库；
//! 该在线接口只翻译项目简介（`translated`），不提供中文项目名，
//! 因此这里只补简介，名称仍用源站原文。

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Default)]
pub struct TranslatedProject {
    /// 中文简介
    pub summary: String,
}

fn cache() -> &'static Mutex<HashMap<String, Option<TranslatedProject>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<TranslatedProject>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(source: &str, id: &str) -> String {
    format!("{source}:{id}")
}

/// 批量取中文名；未命中缓存的会更新的，失败结果也会缓存避免重复请求。
pub async fn batch(
    client: &reqwest::Client,
    source: &str,
    ids: &[String],
) -> HashMap<String, TranslatedProject> {
    let mut result = HashMap::new();
    let mut pending: Vec<String> = Vec::new();
    {
        let guard = cache().lock().unwrap();
        for id in ids {
            match guard.get(&cache_key(source, id)) {
                Some(Some(value)) => {
                    result.insert(id.clone(), value.clone());
                }
                Some(None) => {}
                None => pending.push(id.clone()),
            }
        }
    }
    if pending.is_empty() {
        return result;
    }

    let mut set = tokio::task::JoinSet::new();
    for id in pending.into_iter().take(40) {
        let client = client.clone();
        let source = source.to_string();
        set.spawn(async move {
            let value = fetch(&client, &source, &id).await;
            (id, value)
        });
    }
    while let Some(joined) = set.join_next().await {
        let Ok((id, value)) = joined else {
            continue;
        };
        cache()
            .lock()
            .unwrap()
            .insert(cache_key(source, &id), value.clone());
        if let Some(value) = value {
            result.insert(id, value);
        }
    }
    result
}

async fn fetch(
    client: &reqwest::Client,
    source: &str,
    id: &str,
) -> Option<TranslatedProject> {
    let url = format!("https://mod.mcimirror.top/translate/{source}/{id}");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(6))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let value: serde_json::Value = resp.json().await.ok()?;
    // 实际返回：{"modid":238222,"translated":"查看物品和配方","original":"View Items and Recipes"}
    // translated 是简介（summary）的中文，不是项目名
    let summary = extract(&value, &["translated"])?;
    let summary = summary.trim().to_string();
    if summary.is_empty() {
        return None;
    }
    Some(TranslatedProject { summary })
}

fn extract(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    let obj = value.as_object()?;
    for key in keys {
        if let Some(text) = obj.get(*key).and_then(|value| value.as_str()) {
            if !text.trim().is_empty() {
                return Some(text.to_string());
            }
        }
    }
    // 接口可能把结果包在 translated / data 里，递归一层
    for wrapper in ["translated", "data", "result"] {
        if let Some(inner) = obj.get(wrapper) {
            if let Some(found) = extract(inner, keys) {
                return Some(found);
            }
        }
    }
    None
}
