use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: Option<String>,
    #[serde(rename = "releaseTime")]
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionSummary {
    pub id: String,
    pub kind: String,
    pub release_time: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDetail {
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    pub assets: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndexRef>,
    pub downloads: Option<VersionDownloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    pub logging: Option<Logging>,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
}

impl VersionDetail {
    pub fn required_java(&self) -> u32 {
        self.java_version
            .as_ref()
            .map(|j| j.major_version)
            .unwrap_or(8)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
    #[serde(rename = "totalSize", default)]
    pub total_size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDownloads {
    pub client: Option<DownloadRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadRef {
    pub sha1: String,
    pub size: u64,
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<Extract>,
    /// Fabric / Quilt 清单直接给出 Maven 仓库根地址
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadRef>,
    pub classifiers: Option<HashMap<String, DownloadRef>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Extract {
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgValue>,
    #[serde(default)]
    pub jvm: Vec<ArgValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: RawArgValue,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RawArgValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Logging {
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: DownloadRef,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexFile {
    #[serde(default)]
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeAll {
    #[serde(rename = "windows-x64")]
    pub windows_x64: Option<HashMap<String, Vec<RuntimeEntry>>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeEntry {
    pub manifest: RuntimeManifestRef,
    pub version: RuntimeVersion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeManifestRef {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeVersion {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeManifest {
    pub files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeFile {
    #[serde(rename = "type")]
    pub kind: String,
    pub downloads: Option<RuntimeDownloads>,
    #[serde(default)]
    pub executable: bool,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeDownloads {
    pub raw: DownloadRef,
}
