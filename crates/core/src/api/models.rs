use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Modrinth API: loader info from /v2/tag/loader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loader {
    pub icon: String,
    pub name: String,
    pub supported_project_types: Vec<String>,
}

/// Modrinth API: game version info from /v2/tag/game_version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersion {
    pub version: String,
    pub version_type: String,
    pub date: String,
    pub major: bool,
}

/// Modrinth API: a file within a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    #[serde(default)]
    pub file_type: Option<FileType>,
}

/// Modrinth API: sha512 and sha1 hashes for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    pub sha512: String,
    pub sha1: String,
}

/// Modrinth API: file type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum FileType {
    RequiredResourcePack,
    OptionalResourcePack,
    SourcesJar,
    DevJar,
    JavadocJar,
    Signature,
    Unknown,
}

/// Modrinth API: a version dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    #[serde(default)]
    pub dependency_type: Option<DependencyType>,
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
}

/// Modrinth API: dependency type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

/// Modrinth API: version type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Beta,
    Alpha,
}

/// Modrinth API: version status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
    Unknown,
}

/// Modrinth API: a Version (from /v2/version/{id}, version_files, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub name: Option<String>,
    pub version_number: Option<String>,
    pub changelog: Option<String>,
    pub changelog_url: Option<String>,
    pub date_published: DateTime<Utc>,
    pub downloads: u64,
    pub version_type: VersionType,
    pub status: VersionStatus,
    pub requested_status: Option<VersionStatus>,
    pub featured: bool,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

/// Modrinth API: search result hit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub project_type: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub categories: Vec<String>,
    pub display_categories: Vec<String>,
    pub versions: Vec<String>,
    pub latest_version: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
    pub license: String,
    pub client_side: String,
    pub server_side: String,
    pub icon_url: Option<String>,
    pub color: Option<u32>,
    pub thread_id: String,
    pub monetization_status: String,
    pub gallery: Vec<String>,
    pub featured_gallery: Option<String>,
}

/// Modrinth API: search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

/// Request body for POST /v2/version_files
#[derive(Debug, Clone, Serialize)]
pub struct VersionFilesRequest {
    pub hashes: Vec<String>,
    pub algorithm: String,
}

/// Request body for POST /v2/version_files/update
#[derive(Debug, Clone, Serialize)]
pub struct VersionFilesUpdateRequest {
    pub hashes: Vec<String>,
    pub algorithm: String,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

/// Response from POST /v2/version_files: hash → Version
pub type VersionFilesResponse = HashMap<String, Version>;

/// Response from POST /v2/version_files/update: hash → Version (only updated ones)
pub type VersionFilesUpdateResponse = HashMap<String, Version>;

/// The hash algorithm to use
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashAlgorithm {
    Sha1,
    Sha512,
}

impl HashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Sha512 => "sha512",
        }
    }
}
