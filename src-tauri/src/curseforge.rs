use crate::config::Config;
use crate::download::{self, FileTask, ProgressState};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::AppHandle;

const BASE: &str = "https://api.curse.tools/v1/cf";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLogo {
    #[serde(default)]
    thumbnail_url: String,
    #[serde(default)]
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CfAuthor {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: i64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    file_length: u64,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    release_type: i32,
    #[serde(default)]
    dependencies: Vec<CfDependency>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfDependency {
    #[serde(default)]
    mod_id: i64,
    /// 3 = 必需前置，2 = 可选，1 = 内置
    #[serde(default)]
    relation_type: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfCategory {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    download_count: u64,
    #[serde(default)]
    class_id: i64,
    #[serde(default)]
    date_modified: String,
    #[serde(default)]
    categories: Vec<CfCategory>,
    logo: Option<CfLogo>,
    #[serde(default)]
    authors: Vec<CfAuthor>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfPagination {
    #[serde(default)]
    total_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct CfSearchResponse {
    #[serde(default)]
    data: Vec<CfMod>,
    pagination: Option<CfPagination>,
}

#[derive(Debug, Clone, Deserialize)]
struct CfFilesResponse {
    #[serde(default)]
    data: Vec<CfFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CfProject {
    pub source: String,
    pub id: String,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: u64,
    pub class_id: i64,
    pub logo_url: String,
    pub author: String,
    pub updated: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CfSearchResult {
    pub items: Vec<CfProject>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CfFileView {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub file_date: String,
    pub size: u64,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub release_type: i32,
    /// 必需前置的项目 ID，供界面显示「N 项前置」
    pub dependencies: Vec<String>,
}

fn loader_id(loader: &str) -> Option<&'static str> {
    match loader {
        "forge" => Some("1"),
        "fabric" => Some("4"),
        "quilt" => Some("5"),
        "neoforge" => Some("6"),
        _ => None,
    }
}

pub async fn search(
    client: &reqwest::Client,
    class_id: i64,
    query: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    index: u32,
    page_size: u32,
    sort_field: u32,
) -> Result<CfSearchResult> {
    let mut url = reqwest::Url::parse(&format!("{BASE}/mods/search"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("gameId", "432")
            .append_pair("classId", &class_id.to_string())
            .append_pair("index", &index.to_string())
            .append_pair("pageSize", &page_size.to_string())
            .append_pair("sortField", &sort_field.to_string())
            .append_pair("sortOrder", "desc");
        if !query.trim().is_empty() {
            pairs.append_pair("searchFilter", query.trim());
        }
        if let Some(version) = game_version {
            if !version.is_empty() {
                pairs.append_pair("gameVersion", version);
            }
        }
        if let Some(loader) = loader.and_then(loader_id) {
            pairs.append_pair("modLoaderType", loader);
        }
    }

    let response: CfSearchResponse =
        download::get_json(client, url.as_str(), false).await?;
    let items = response.data.into_iter().map(to_project).collect();
    Ok(CfSearchResult {
        items,
        total: response
            .pagination
            .map(|p| p.total_count)
            .unwrap_or(0),
    })
}

fn to_project(project: CfMod) -> CfProject {
    CfProject {
        source: "curseforge".into(),
        id: project.id.to_string(),
        name: project.name,
        slug: project.slug,
        summary: project.summary,
        download_count: project.download_count,
        class_id: project.class_id,
        logo_url: project
            .logo
            .map(|logo| {
                if logo.thumbnail_url.is_empty() {
                    logo.url
                } else {
                    logo.thumbnail_url
                }
            })
            .unwrap_or_default(),
        author: project
            .authors
            .first()
            .map(|author| author.name.clone())
            .unwrap_or_default(),
        updated: project.date_modified,
        categories: project
            .categories
            .into_iter()
            .map(|category| category.name)
            .filter(|name| !name.is_empty())
            .collect(),
    }
}

/// 按项目 ID 取详情，用于下载页展示前置资源。
pub async fn projects(client: &reqwest::Client, ids: &[i64]) -> Vec<CfProject> {
    #[derive(Deserialize)]
    struct CfModResponse {
        data: CfMod,
    }
    let mut result = Vec::new();
    for id in ids.iter().take(10) {
        let url = format!("{BASE}/mods/{id}");
        if let Ok(response) = download::get_json::<CfModResponse>(client, &url, false).await {
            result.push(to_project(response.data));
        }
    }
    result
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfGameVersion {
    #[serde(default)]
    version_string: String,
    #[serde(default)]
    date_modified: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CfGameVersionsResponse {
    #[serde(default)]
    data: Vec<CfGameVersion>,
}

/// 供版本下拉框使用的正式版列表，按发布时间倒序。
pub async fn game_versions(client: &reqwest::Client) -> Result<Vec<String>> {
    let response: CfGameVersionsResponse = download::get_json(
        client,
        &format!("{BASE}/minecraft/version"),
        false,
    )
    .await?;
    let mut list: Vec<CfGameVersion> = response
        .data
        .into_iter()
        .filter(|entry| {
            let value = &entry.version_string;
            value.contains('.')
                && value
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.')
        })
        .collect();
    list.sort_by(|a, b| b.date_modified.cmp(&a.date_modified));
    Ok(list
        .into_iter()
        .map(|entry| entry.version_string)
        .take(60)
        .collect())
}

pub async fn files(
    client: &reqwest::Client,
    mod_id: i64,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<CfFileView>> {
    let mut url = reqwest::Url::parse(&format!("{BASE}/mods/{mod_id}/files"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("pageSize", "50");
        if let Some(version) = game_version {
            if !version.is_empty() {
                pairs.append_pair("gameVersion", version);
            }
        }
        if let Some(loader) = loader.and_then(loader_id) {
            pairs.append_pair("modLoaderType", loader);
        }
    }
    let response: CfFilesResponse = download::get_json(client, url.as_str(), false).await?;
    let mut files: Vec<CfFileView> = response
        .data
        .into_iter()
        .map(|file| CfFileView {
            id: file.id.to_string(),
            name: if file.display_name.is_empty() {
                file.file_name.clone()
            } else {
                file.display_name
            },
            file_name: file.file_name,
            file_date: file.file_date,
            size: file.file_length,
            download_url: file.download_url,
            game_versions: file.game_versions,
            release_type: file.release_type,
            dependencies: file
                .dependencies
                .into_iter()
                .filter(|dep| dep.relation_type == 3 && dep.mod_id > 0)
                .map(|dep| dep.mod_id.to_string())
                .collect(),
        })
        .collect();
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    files.truncate(25);
    Ok(files)
}

pub async fn file_download(
    client: &reqwest::Client,
    mod_id: i64,
    file_id: i64,
) -> Result<(String, String)> {
    #[derive(Deserialize)]
    struct CfFileResponse {
        data: CfFile,
    }
    let url = format!("{BASE}/mods/{mod_id}/files/{file_id}");
    let response: CfFileResponse = download::get_json(client, &url, false).await?;
    let file = response.data;
    let download_url = file
        .download_url
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| anyhow!("作者未开放第三方下载"))?;
    let name = if file.file_name.is_empty() {
        format!("{mod_id}-{file_id}.jar")
    } else {
        file.file_name
    };
    Ok((download_url, name))
}

#[derive(Debug, Serialize)]
pub struct InstallOutcome {
    pub path: String,
    pub dependencies: crate::moddeps::DependencyReport,
}

pub async fn install_file(
    app: &AppHandle,
    client: &reqwest::Client,
    cfg: &Config,
    instance_id: &str,
    kind: &str,
    url: &str,
    file_name: &str,
    progress: Arc<ProgressState>,
) -> Result<InstallOutcome> {
    let name = Path::new(file_name)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "download.bin".to_string());
    let dir = crate::instance::content_dir(cfg, instance_id, kind)?;
    let dest = dir.join(&name);
    let task = FileTask {
        url: url.to_string(),
        dest: dest.clone(),
        sha1: None,
        size: 0,
        label: name,
    };
    download::run_tasks(app, client, vec![task], 1, false, progress.clone()).await?;

    // 模组装好后顺手补齐它声明的必需前置，装不上的留给前端提示
    let mut dependencies = crate::moddeps::DependencyReport::default();
    if matches!(kind, "mods" | "mod") {
        progress.set_message("检查模组前置…");
        download::emit_now(app, &progress);
        dependencies = crate::moddeps::install_missing(
            app,
            client,
            cfg,
            instance_id,
            &dest,
            progress.clone(),
        )
        .await;
    }

    // 前置检查结束后必须改写进度文案，否则面板会一直停在「检查模组前置…」
    let done = if !dependencies.installed.is_empty() {
        format!("安装完成，已补齐前置 {}", dependencies.installed.join("、"))
    } else if !dependencies.unresolved.is_empty() {
        format!("安装完成，缺少前置 {}", dependencies.unresolved.join("、"))
    } else {
        "安装完成".to_string()
    };
    progress.set_message(done);

    Ok(InstallOutcome {
        path: dest.to_string_lossy().to_string(),
        dependencies,
    })
}
