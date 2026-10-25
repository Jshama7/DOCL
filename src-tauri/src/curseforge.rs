use crate::config::Config;
use crate::download::{self, FileTask, ProgressState};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::AppHandle;

const BASE: &str = "https://api.curse.tools/v1/cf";
/// 备用 API 源（MCIM 国内镜像，PCL 也用它），海外代理被墙或拒绝时兜底
const MIRROR_BASE: &str = "https://mod.mcimirror.top/curseforge/v1";

/// 依次尝试两个 API 源，返回第一个成功的结果。
/// 默认镜像优先：curse.tools 在国内经常完全连不上（实测 HTTP 000，等满超时才失败）。
async fn get_json_api<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    path: &str,
    prefer_mirror: bool,
) -> Result<T> {
    let bases: [&str; 2] = if prefer_mirror {
        [MIRROR_BASE, BASE]
    } else {
        [BASE, MIRROR_BASE]
    };
    let mut last_error: Option<anyhow::Error> = None;
    for base in bases {
        let url = format!("{base}{path}");
        // 单个源最多等 12 秒，避免被墙的源拖满整轮超时
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(12),
            download::get_json::<T>(client, &url, false),
        )
        .await;
        match outcome {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(err)) => {
                #[cfg(debug_assertions)]
                eprintln!("[curseforge] {base} 请求 {path} 失败: {err}");
                last_error = Some(err);
            }
            Err(_) => {
                #[cfg(debug_assertions)]
                eprintln!("[curseforge] {base} 请求 {path} 超时");
                last_error = Some(anyhow!("CurseForge 接口超时"));
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    Err(last_error.unwrap_or_else(|| anyhow!("CurseForge 接口不可用")))
}

/// 把基于 BASE 拼好的完整 URL 转成相对路径，交给多源请求。
fn relative_path(full: &str) -> String {
    full.trim_start_matches(BASE).to_string()
}

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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CfLinks {
    #[serde(default)]
    website_url: String,
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
    #[serde(default)]
    links: Option<CfLinks>,
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
    /// 官方项目网页地址，前端「转到官网」按钮直接打开
    pub website_url: String,
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

#[derive(Debug, Clone, Deserialize)]
struct CfCategoryListResponse {
    #[serde(default)]
    data: Vec<CfCategoryRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct CfCategoryRaw {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
}

/// CurseForge 分类（类型筛选下拉用）。
#[derive(Debug, Clone, Serialize)]
pub struct CfCategoryView {
    pub id: i64,
    pub name: String,
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
    category_id: Option<i64>,
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
        if let Some(category_id) = category_id {
            pairs.append_pair("categoryId", &category_id.to_string());
        }
    }

    let response: CfSearchResponse =
        get_json_api(client, &relative_path(url.as_str()), true).await?;
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
    let links = project.links.unwrap_or_default();
    // CF 模组的网页路径是 mc-mods，mods 路径会 404；其余分类由 websiteUrl 官方地址兜底
    let website_url = if links.website_url.is_empty() {
        format!("https://www.curseforge.com/minecraft/mc-mods/{}", project.slug)
    } else {
        links.website_url
    };
    CfProject {
        source: "curseforge".into(),
        id: project.id.to_string(),
        name: project.name,
        slug: project.slug,
        website_url,
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
        let path = format!("/mods/{id}");
        if let Ok(response) = get_json_api::<CfModResponse>(client, &path, true).await {
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

/// 拉取某资源大类下的分类列表，供类型筛选下拉使用。
pub async fn categories(client: &reqwest::Client, class_id: i64) -> Result<Vec<CfCategoryView>> {
    let mut url = reqwest::Url::parse(&format!("{BASE}/categories"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("gameId", "432")
            .append_pair("classId", &class_id.to_string());
    }
    let response: CfCategoryListResponse =
        get_json_api(client, &relative_path(url.as_str()), true).await?;
    let mut list: Vec<CfCategoryView> = response
        .data
        .into_iter()
        .map(|raw| CfCategoryView {
            id: raw.id,
            name: raw.name,
        })
        .collect();
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(list)
}

/// 供版本下拉框使用的正式版列表，按发布时间倒序。
pub async fn game_versions(client: &reqwest::Client) -> Result<Vec<String>> {
    let response: CfGameVersionsResponse =
        get_json_api(client, "/minecraft/version", true).await?;
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
    let response: CfFilesResponse = get_json_api(client, &relative_path(url.as_str()), true).await?;
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

/// 读取文件详情并取出下载直链。
/// 403/404 属于服务端明确拒绝，原地重试没有意义，像 PCL 一样直接换源：
/// 先试配置偏好的源，失败再换另一个；每个源内部仍有一次重试。
pub async fn file_download(
    client: &reqwest::Client,
    mod_id: i64,
    file_id: i64,
    prefer_mirror: bool,
) -> Result<(String, String)> {
    #[derive(Deserialize)]
    struct CfFileResponse {
        data: CfFile,
    }

    let path = format!("/mods/{mod_id}/files/{file_id}");
    let file = get_json_api::<CfFileResponse>(client, &path, prefer_mirror)
        .await?
        .data;

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
