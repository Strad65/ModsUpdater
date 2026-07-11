pub mod models;

use crate::api::models::*;
use crate::error::{ApiError, CoreError, CoreResult};

use reqwest::{Client, StatusCode, Url};
use std::time::Duration;
use tracing::{debug, info};

const BASE_URL: &str = "https://api.modrinth.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RETRIES: u32 = 3;

/// Client for interacting with the Modrinth API v2.
pub struct ModrinthClient {
    client: Client,
    base_url: Url,
}

impl ModrinthClient {
    /// Create a new ModrinthClient with the given User-Agent string.
    ///
    /// Modrinth requires a unique User-Agent header. Format:
    /// `username/project/version (contact@domain.com)`
    pub fn new(user_agent: &str) -> CoreResult<Self> {
        let client = Client::builder()
            .user_agent(user_agent)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| CoreError::Config(format!("Failed to build HTTP client: {}", e)))?;

        let base_url = Url::parse(BASE_URL).expect("BASE_URL is a valid URL");

        Ok(Self { client, base_url })
    }

    // ── Tag Endpoints ────────────────────────────────────────────────

    /// Get the list of available mod loaders (fabric, forge, quilt, etc.)
    pub async fn get_loaders(&self) -> CoreResult<Vec<Loader>> {
        let url = self.base_url.join("/v2/tag/loader").unwrap();
        self.get_json::<Vec<Loader>>(url).await
    }

    /// Get the list of Minecraft versions.
    pub async fn get_game_versions(&self) -> CoreResult<Vec<GameVersion>> {
        let url = self.base_url.join("/v2/tag/game_version").unwrap();
        self.get_json::<Vec<GameVersion>>(url).await
    }

    /// Get the list of categories.
    pub async fn get_categories(&self) -> CoreResult<Vec<Category>> {
        let url = self.base_url.join("/v2/tag/category").unwrap();
        self.get_json::<Vec<Category>>(url).await
    }

    // ── Search ───────────────────────────────────────────────────────

    /// Search for projects by query string.
    pub async fn search(
        &self,
        query: &str,
        facets: Option<&str>,
        index: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> CoreResult<SearchResponse> {
        let mut url = self.base_url.join("/v2/search").unwrap();
        {
            let mut params = url.query_pairs_mut();
            params.append_pair("query", query);
            if let Some(f) = facets {
                params.append_pair("facets", f);
            }
            if let Some(i) = index {
                params.append_pair("index", i);
            }
            params.append_pair("offset", &offset.to_string());
            params.append_pair("limit", &limit.to_string());
        }
        self.get_json::<SearchResponse>(url).await
    }

    /// Search for a project by name and return the best match.
    pub async fn search_project(&self, name: &str) -> CoreResult<Option<SearchHit>> {
        let response = self.search(name, None, Some("relevance"), 0, 5).await?;
        let name_lower = name.to_lowercase();

        // Prefer exact title or slug match, then fall back to the first result
        let best = response.hits.into_iter().find(|h| {
            h.title.to_lowercase() == name_lower || h.slug.to_lowercase() == name_lower
        });
        Ok(best)
    }

    // ── Version Files (core update logic) ────────────────────────────

    /// Batch lookup: given file hashes, return the current Version for each.
    /// Uses POST /v2/version_files
    pub async fn get_versions_by_hashes(
        &self,
        hashes: &[String],
        algorithm: HashAlgorithm,
    ) -> CoreResult<VersionFilesResponse> {
        let url = self.base_url.join("/v2/version_files").unwrap();
        let body = VersionFilesRequest {
            hashes: hashes.to_vec(),
            algorithm: algorithm.as_str().to_string(),
        };
        info!(
            "Looking up {} hashes via version_files (algorithm={})",
            hashes.len(),
            algorithm.as_str()
        );
        self.post_json(url, &body).await
    }

    /// Batch update check: given file hashes, loader, and game version,
    /// return new Versions for mods that have updates available.
    /// Uses POST /v2/version_files/update
    ///
    /// Only returns entries for hashes that HAVE updates.
    /// Hashes that are already up-to-date will NOT appear in the response.
    pub async fn check_updates(
        &self,
        hashes: &[String],
        algorithm: HashAlgorithm,
        loaders: &[String],
        game_versions: &[String],
    ) -> CoreResult<VersionFilesUpdateResponse> {
        let url = self.base_url.join("/v2/version_files/update").unwrap();
        let body = VersionFilesUpdateRequest {
            hashes: hashes.to_vec(),
            algorithm: algorithm.as_str().to_string(),
            loaders: loaders.to_vec(),
            game_versions: game_versions.to_vec(),
        };
        info!(
            "Checking updates for {} hashes (loader={:?}, game_versions={:?})",
            hashes.len(),
            loaders,
            game_versions
        );
        self.post_json(url, &body).await
    }

    /// Download a file from a URL, returning the raw bytes.
    /// The URL is typically a Modrinth CDN URL from a VersionFile.
    pub async fn download_file(&self, url: &str) -> CoreResult<bytes::Bytes> {
        info!("Downloading: {}", url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CoreError::Http(e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(CoreError::Api(ApiError::ApiError {
                status: status.as_u16(),
                message: format!("Download failed with status {}", status),
            }));
        }

        response
            .bytes()
            .await
            .map_err(|e| CoreError::Http(e))
    }

    /// Download a file with progress callback.
    /// The callback receives (bytes_downloaded, total_bytes).
    pub async fn download_file_with_progress<F>(
        &self,
        url: &str,
        on_progress: F,
    ) -> CoreResult<bytes::Bytes>
    where
        F: Fn(u64, u64) + Send + Sync,
    {
        use futures_util::StreamExt;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| CoreError::Http(e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(CoreError::Api(ApiError::ApiError {
                status: status.as_u16(),
                message: format!("Download failed with status {}", status),
            }));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut bytes_vec = Vec::new();

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| CoreError::Http(e))?;
            downloaded += chunk.len() as u64;
            bytes_vec.extend_from_slice(&chunk);
            on_progress(downloaded, total_size);
        }

        Ok(bytes::Bytes::from(bytes_vec))
    }

    // ── Internal helpers ─────────────────────────────────────────────

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: Url) -> CoreResult<T> {
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| CoreError::Http(e))?;

        self.handle_json_response(response).await
    }

    async fn post_json<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        url: Url,
        body: &B,
    ) -> CoreResult<T> {
        let mut retries = 0;
        loop {
            let response = self
                .client
                .post(url.clone())
                .json(body)
                .send()
                .await
                .map_err(|e| CoreError::Http(e))?;

            let status = response.status();

            // Handle rate limiting
            if status == StatusCode::TOO_MANY_REQUESTS {
                if retries < MAX_RETRIES {
                    retries += 1;
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(Duration::from_secs)
                        .unwrap_or(Duration::from_secs(1));
                    debug!("Rate limited, retrying in {:?} (attempt {}/{})", retry_after, retries, MAX_RETRIES);
                    tokio::time::sleep(retry_after).await;
                    continue;
                }
                return Err(CoreError::Api(ApiError::RateLimited { retry_after: None }));
            }

            return self.handle_json_response_inner(status, response).await;
        }
    }

    async fn handle_json_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> CoreResult<T> {
        let status = response.status();
        self.handle_json_response_inner(status, response).await
    }

    async fn handle_json_response_inner<T: serde::de::DeserializeOwned>(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> CoreResult<T> {
        if status.is_success() {
            let bytes = response.bytes().await.map_err(|e| CoreError::Http(e))?;
            serde_json::from_slice(&bytes).map_err(CoreError::Serde)
        } else if status == StatusCode::NOT_FOUND {
            Err(CoreError::Api(ApiError::NotFound(
                "Resource not found".to_string(),
            )))
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(CoreError::Api(ApiError::ApiError {
                status: status.as_u16(),
                message,
            }))
        }
    }
}

/// Category from /v2/tag/category (simplified)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Category {
    pub icon: String,
    pub name: String,
    pub project_type: String,
    pub header: String,
}
