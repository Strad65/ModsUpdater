use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::api::models::{HashAlgorithm, Version};
use crate::api::ModrinthClient;
use crate::error::{CoreError, CoreResult};
use crate::scanner::{self, LocalMod};

// ── Data Structures ──────────────────────────────────────────────────

/// Information about a single mod: local file + current/update versions from API.
#[derive(Debug, Clone)]
pub struct ModInfo {
    pub local: LocalMod,
    /// The current version info from the API (via version_files lookup)
    pub current_version: Option<Version>,
    /// The update version from the API (via version_files/update)
    pub update_version: Option<Version>,
}

/// Result of a single update operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub filename: String,
    pub old_path: PathBuf,
    /// Path where the new file was saved (if successful)
    pub new_path: Option<PathBuf>,
    pub project_name: Option<String>,
    pub old_version: Option<String>,
    pub new_version_name: Option<String>,
    pub new_version_number: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// Overall report of an update run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReport {
    pub total_scanned: usize,
    pub total_updates_available: usize,
    pub total_updated: usize,
    pub total_failed: usize,
    pub total_up_to_date: usize,
    pub results: Vec<UpdateResult>,
    pub loader: String,
    pub game_version: String,
    pub timestamp: DateTime<Utc>,
}

impl UpdateReport {
    /// Generate a markdown report string.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Modrinth Update Report\n\n");
        md.push_str(&format!(
            "**Generated:** {}\n\n",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!(
            "**Loader:** {} &nbsp;|&nbsp; **Game Version:** {}\n\n",
            self.loader, self.game_version
        ));

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str("| Category | Count |\n");
        md.push_str("|----------|-------|\n");
        md.push_str(&format!(
            "| Mods scanned | {} |\n",
            self.total_scanned
        ));
        md.push_str(&format!(
            "| Updates available | {} |\n",
            self.total_updates_available
        ));
        md.push_str(&format!(
            "| Successfully updated | {} |\n",
            self.total_updated
        ));
        md.push_str(&format!(
            "| Failed | {} |\n",
            self.total_failed
        ));
        md.push_str(&format!(
            "| Already up-to-date | {} |\n",
            self.total_up_to_date
        ));
        md.push('\n');

        // Updated mods
        let updated: Vec<_> = self.results.iter().filter(|r| r.success).collect();
        if !updated.is_empty() {
            md.push_str("## Updated Mods\n\n");
            md.push_str("| Mod | Old Version | New Version |\n");
            md.push_str("|-----|-------------|-------------|\n");
            for r in &updated {
                let name = r
                    .project_name
                    .as_deref()
                    .unwrap_or(&r.filename);
                let old = r.old_version.as_deref().unwrap_or("?");
                let new = r.new_version_number.as_deref().unwrap_or("?");
                md.push_str(&format!("| {} | {} | {} |\n", name, old, new));
            }
            md.push('\n');
        }

        // Failed mods
        let failed: Vec<_> = self.results.iter().filter(|r| !r.success).collect();
        if !failed.is_empty() {
            md.push_str("## Failed Updates\n\n");
            md.push_str("| Mod | Error |\n");
            md.push_str("|-----|-------|\n");
            for r in &failed {
                let name = r
                    .project_name
                    .as_deref()
                    .unwrap_or(&r.filename);
                let error = r.error.as_deref().unwrap_or("Unknown error");
                md.push_str(&format!("| {} | {} |\n", name, error));
            }
            md.push('\n');
        }

        md
    }
}

// ── Core Update Logic ────────────────────────────────────────────────

/// Analyzes local mods against the Modrinth API to find which mods have updates.
///
/// Returns a Vec of ModInfo with current and update version data.
pub async fn analyze_mods(
    client: &ModrinthClient,
    local_mods: &[LocalMod],
    loaders: &[String],
    game_versions: &[String],
) -> CoreResult<Vec<ModInfo>> {
    if local_mods.is_empty() {
        return Ok(Vec::new());
    }

    // Use SHA-512 hashes (recommended by Modrinth)
    let hashes: Vec<String> = local_mods.iter().map(|m| m.sha512.clone()).collect();

    // Step 1: Look up current versions for all mods
    let current_versions = client
        .get_versions_by_hashes(&hashes, HashAlgorithm::Sha512)
        .await?;

    info!(
        "Resolved {} / {} mods to current versions",
        current_versions.len(),
        local_mods.len()
    );

    // Step 2: Check for updates
    let update_versions = client
        .check_updates(&hashes, HashAlgorithm::Sha512, loaders, game_versions)
        .await?;

    info!(
        "Found {} update(s) available out of {} mod(s)",
        update_versions.len(),
        local_mods.len()
    );

    // Step 3: Build ModInfo for each local mod
    let mod_infos: Vec<ModInfo> = local_mods
        .iter()
        .map(|local| {
            let current = current_versions.get(&local.sha512).cloned();
            let update = update_versions.get(&local.sha512).cloned();
            ModInfo {
                local: local.clone(),
                current_version: current,
                update_version: update,
            }
        })
        .collect();

    Ok(mod_infos)
}

/// Download a single update and save it to `output_dir`.
///
/// The downloaded file is saved as `output_dir/<api_filename>` using the
/// filename from the Modrinth API (which includes the correct version number).
/// The original local file is **never** modified.
pub async fn install_update(
    client: &ModrinthClient,
    mod_info: &ModInfo,
    output_dir: &Path,
) -> CoreResult<UpdateResult> {
    let update_version = mod_info
        .update_version
        .as_ref()
        .ok_or_else(|| CoreError::Update("No update version available".to_string()))?;

    // Find the primary file to download
    let primary_file = update_version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| update_version.files.first())
        .ok_or_else(|| {
            CoreError::Update(format!(
                "No downloadable file found for update of {}",
                mod_info.local.filename
            ))
        })?;

    let new_filename = &primary_file.filename;

    info!(
        "Downloading update for {}: {} -> {}",
        mod_info.local.filename,
        mod_info
            .current_version
            .as_ref()
            .and_then(|v| v.version_number.as_deref())
            .unwrap_or("?"),
        update_version
            .version_number
            .as_deref()
            .unwrap_or("?")
    );

    // Download the file
    let data = client.download_file(&primary_file.url).await?;

    // Verify hash (SHA-512)
    let actual_sha512 = scanner::hash_bytes_sha512(&data);
    if actual_sha512 != primary_file.hashes.sha512 {
        return Err(CoreError::HashMismatch {
            filename: new_filename.clone(),
            expected: primary_file.hashes.sha512.clone(),
            actual: actual_sha512,
        });
    }

    debug!(
        "Hash verified for {}: sha512={:.16}...",
        new_filename, actual_sha512
    );

    // Ensure output directory exists
    std::fs::create_dir_all(output_dir).map_err(|e| {
        CoreError::Update(format!(
            "Failed to create output directory {:?}: {}",
            output_dir, e
        ))
    })?;

    let new_path = output_dir.join(new_filename);

    // If the target file already exists (e.g. from a previous update), remove it first
    if new_path.exists() {
        std::fs::remove_file(&new_path).ok();
    }

    // Use tempfile in the output directory so persist (rename) stays on same filesystem
    let temp_file = tempfile::NamedTempFile::new_in(output_dir).map_err(|e| {
        CoreError::Update(format!("Failed to create temp file: {}", e))
    })?;

    std::fs::write(temp_file.path(), &data).map_err(|e| {
        CoreError::Update(format!("Failed to write downloaded file: {}", e))
    })?;

    // Move temp file to output directory
    temp_file.persist(&new_path).map_err(|e| {
        CoreError::Update(format!(
            "Failed to save {} to {:?}: {}",
            new_filename, output_dir, e.error
        ))
    })?;

    info!("Saved updated mod: {:?}", new_path);

    let project_name = mod_info
        .current_version
        .as_ref()
        .map(|_| {
            mod_info
                .local
                .filename
                .rsplitn(2, '-')
                .last()
                .unwrap_or(&mod_info.local.filename)
                .trim_end_matches(".jar")
                .to_string()
        });

    Ok(UpdateResult {
        filename: mod_info.local.filename.clone(),
        old_path: mod_info.local.path.clone(),
        new_path: Some(new_path),
        project_name,
        old_version: mod_info
            .current_version
            .as_ref()
            .and_then(|v| v.version_number.clone()),
        new_version_name: update_version.name.clone(),
        new_version_number: update_version.version_number.clone(),
        success: true,
        error: None,
    })
}

/// Run the full update process: analyze + download all updates.
///
/// Updated mods are saved to `output_dir` using their API-provided filenames.
/// Original files are **never** modified.
///
/// Returns a complete UpdateReport.
pub async fn run_updates(
    client: &ModrinthClient,
    local_mods: &[LocalMod],
    loaders: &[String],
    game_versions: &[String],
    dry_run: bool,
    output_dir: &Path,
) -> CoreResult<UpdateReport> {
    let timestamp = Utc::now();

    // Analyze all mods
    let mod_infos = analyze_mods(client, local_mods, loaders, game_versions).await?;

    let total_scanned = mod_infos.len();
    let updates_available: Vec<&ModInfo> = mod_infos
        .iter()
        .filter(|m| m.update_version.is_some())
        .collect();
    let total_updates_available = updates_available.len();
    let up_to_date = total_scanned - total_updates_available;

    if dry_run {
        info!("Dry run: {} update(s) would be downloaded", total_updates_available);
        let results = updates_available
            .iter()
            .map(|m| {
                let update = m.update_version.as_ref().unwrap();
                UpdateResult {
                    filename: m.local.filename.clone(),
                    old_path: m.local.path.clone(),
                    new_path: None,
                    project_name: Some(
                        m.local
                            .filename
                            .rsplitn(2, '-')
                            .last()
                            .unwrap_or(&m.local.filename)
                            .trim_end_matches(".jar")
                            .to_string(),
                    ),
                    old_version: m.current_version.as_ref().and_then(|v| v.version_number.clone()),
                    new_version_name: update.name.clone(),
                    new_version_number: update.version_number.clone(),
                    success: false,
                    error: Some("Dry run — not downloaded".to_string()),
                }
            })
            .collect();

        return Ok(UpdateReport {
            total_scanned,
            total_updates_available,
            total_updated: 0,
            total_failed: total_updates_available,
            total_up_to_date: up_to_date,
            results,
            loader: loaders.join(", "),
            game_version: game_versions.join(", "),
            timestamp,
        });
    }

    // Download each update
    let mut results = Vec::new();
    let mut total_updated = 0;
    let mut total_failed = 0;

    for mod_info in &updates_available {
        let result = install_update(client, mod_info, output_dir).await;
        match result {
            Ok(r) => {
                if r.success {
                    total_updated += 1;
                } else {
                    total_failed += 1;
                }
                results.push(r);
            }
            Err(e) => {
                total_failed += 1;
                results.push(UpdateResult {
                    filename: mod_info.local.filename.clone(),
                    old_path: mod_info.local.path.clone(),
                    new_path: None,
                    project_name: None,
                    old_version: mod_info
                        .current_version
                        .as_ref()
                        .and_then(|v| v.version_number.clone()),
                    new_version_name: mod_info
                        .update_version
                        .as_ref()
                        .and_then(|v| v.name.clone()),
                    new_version_number: mod_info
                        .update_version
                        .as_ref()
                        .and_then(|v| v.version_number.clone()),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(UpdateReport {
        total_scanned,
        total_updates_available,
        total_updated,
        total_failed,
        total_up_to_date: up_to_date,
        results,
        loader: loaders.join(", "),
        game_version: game_versions.join(", "),
        timestamp,
    })
}

/// Save an UpdateReport to the data directory as JSON.
pub fn save_report(report: &UpdateReport) -> CoreResult<PathBuf> {
    let dir = crate::config::AppConfig::data_dir()?;
    let filename = format!(
        "report-{}.json",
        report.timestamp.format("%Y%m%d-%H%M%S")
    );
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(report).map_err(|e| {
        CoreError::Update(format!("Failed to serialize report: {}", e))
    })?;

    std::fs::write(&path, &json).map_err(|e| {
        CoreError::Update(format!("Failed to write report: {}", e))
    })?;

    // Also save as "latest-report.json" for easy access
    let latest_path = dir.join("latest-report.json");
    std::fs::write(&latest_path, &json).map_err(|e| {
        CoreError::Update(format!("Failed to write latest report: {}", e))
    })?;

    info!("Report saved to {:?}", path);
    Ok(path)
}

/// Load the most recent report from the data directory.
pub fn load_latest_report() -> CoreResult<Option<UpdateReport>> {
    let dir = crate::config::AppConfig::data_dir()?;
    let latest_path = dir.join("latest-report.json");

    if !latest_path.exists() {
        return Ok(None);
    }

    let data = std::fs::read_to_string(&latest_path).map_err(|e| {
        CoreError::Update(format!("Failed to read latest report: {}", e))
    })?;

    let report: UpdateReport = serde_json::from_str(&data).map_err(|e| {
        CoreError::Update(format!("Failed to parse report: {}", e))
    })?;

    Ok(Some(report))
}
