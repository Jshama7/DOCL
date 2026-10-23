use crate::curseforge::{CfFileView, CfProject, CfSearchResult};
use crate::download;
use anyhow::{bail, Result};
use serde::Deserialize;

const BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Deserialize)]
struct MrSearchResponse {
    #[serde(default)]
    hits: Vec<MrHit>,
    #[serde(default)]
    total_hits: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct MrHit {
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    author: String,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    date_modified: String,
    #[serde(default)]
    project_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MrFile {
    #[serde(default)]
    url: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct MrDependency {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    dependency_type: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MrVersion {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    version_type: String,
    #[serde(default)]
    date_published: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    dependencies: Vec<MrDependency>,
    #[serde(default)]
    files: Vec<MrFile>,
}

fn project_type(kind: &str) -> Result<&'static str> {
    match kind {
        "mods" => Ok("mod"),
        "resourcepacks" => Ok("resourcepack"),
        "shaders" => Ok("shader"),
        "modpacks" => Ok("modpack"),
        "datapacks" => Ok("datapack"),
        _ => bail!("该分类暂不支持 Modrinth：{kind}"),
    }
}

fn loader_tag(loader: &str) -> Option<&'static str> {
    match loader {
        "fabric" => Some("fabric"),
        "quilt" => Some("quilt"),
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        _ => None,
    }
}

/// facets 只接受字母数字、点、下划线与连字符，够覆盖版本号和加载器名。
fn clean(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect()
}

pub async fn search(
    client: &reqwest::Client,
    kind: &str,
    query: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    index: u32,
    page_size: u32,
    sort_field: u32,
) -> Result<CfSearchResult> {
    let mut facets: Vec<String> = vec![format!(
        "[\"project_type:{}\"]",
        project_type(kind)?
    )];
    if let Some(version) = game_version.filter(|v| !v.is_empty()) {
        facets.push(format!("[\"versions:{}\"]", clean(version)));
    }
    if let Some(loader) = loader.and_then(loader_tag) {
        facets.push(format!("[\"categories:{loader}\"]"));
    }

    let mut url = reqwest::Url::parse(&format!("{BASE}/search"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("query", query.trim())
            .append_pair("limit", &page_size.to_string())
            .append_pair("offset", &index.to_string())
            .append_pair(
                "index",
                if sort_field == 3 { "updated" } else { "downloads" },
            )
            .append_pair("facets", &format!("[{}]", facets.join(",")));
    }

    let response: MrSearchResponse = download::get_json(client, url.as_str(), false).await?;
    let items = response
        .hits
        .into_iter()
        .filter(|hit| hit.project_type.is_empty() || hit.project_type == project_type(kind).unwrap_or(""))
        .map(|hit| CfProject {
            source: "modrinth".into(),
            id: hit.project_id,
            name: hit.title,
            slug: hit.slug,
            summary: hit.description,
            download_count: hit.downloads,
            class_id: 0,
            logo_url: hit.icon_url.unwrap_or_default(),
            author: hit.author,
            updated: hit.date_modified,
            categories: hit
                .categories
                .into_iter()
                .filter(|name| loader_tag(name).is_none())
                .collect(),
        })
        .collect();
    Ok(CfSearchResult {
        items,
        total: response.total_hits,
    })
}

pub async fn files(
    client: &reqwest::Client,
    project_id: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<CfFileView>> {
    let mut url = reqwest::Url::parse(&format!("{BASE}/project/{project_id}/version"))?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(version) = game_version.filter(|v| !v.is_empty()) {
            pairs.append_pair("game_versions", &format!("[\"{}\"]", clean(version)));
        }
        if let Some(loader) = loader.and_then(loader_tag) {
            pairs.append_pair("loaders", &format!("[\"{loader}\"]"));
        }
    }

    let versions: Vec<MrVersion> = download::get_json(client, url.as_str(), false).await?;
    let mut files = Vec::new();
    for version in versions {
        let file = version
            .files
            .iter()
            .find(|file| file.primary)
            .or_else(|| version.files.first());
        let Some(file) = file else {
            continue;
        };
        files.push(CfFileView {
            id: version.id,
            name: if version.name.is_empty() {
                version.version_number
            } else {
                version.name
            },
            file_name: file.filename.clone(),
            file_date: version.date_published,
            size: file.size,
            download_url: if file.url.is_empty() {
                None
            } else {
                Some(file.url.clone())
            },
            game_versions: version.game_versions,
            release_type: match version.version_type.as_str() {
                "release" => 1,
                "beta" => 2,
                "alpha" => 3,
                _ => 1,
            },
            dependencies: version
                .dependencies
                .into_iter()
                .filter(|dep| dep.dependency_type == "required")
                .filter_map(|dep| dep.project_id)
                .collect(),
        });
    }
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    files.truncate(25);
    Ok(files)
}

#[derive(Debug, Deserialize)]
struct MrProjectDetail {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    date_modified: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    icon_url: Option<String>,
}

/// 按项目 ID 取详情，用于下载页展示前置资源。
pub async fn projects(client: &reqwest::Client, ids: &[String]) -> Result<Vec<CfProject>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let list: Vec<&String> = ids.iter().take(10).collect();
    let encoded = serde_json::to_string(&list)?;
    let mut url = reqwest::Url::parse(&format!("{BASE}/projects"))?;
    url.query_pairs_mut().append_pair("ids", &encoded);
    let projects: Vec<MrProjectDetail> = download::get_json(client, url.as_str(), false).await?;
    Ok(projects
        .into_iter()
        .map(|project| CfProject {
            source: "modrinth".into(),
            id: project.id,
            name: project.title,
            slug: project.slug,
            summary: project.description,
            download_count: project.downloads,
            class_id: 0,
            logo_url: project.icon_url.unwrap_or_default(),
            author: String::new(),
            updated: project.date_modified,
            categories: project
                .categories
                .into_iter()
                .filter(|name| loader_tag(name).is_none())
                .collect(),
        })
        .collect())
}
