use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::error::{CoreError, CoreResult};

/// Represents a local mod file found by the scanner.
#[derive(Debug, Clone)]
pub struct LocalMod {
    /// Absolute path to the mod file
    pub path: PathBuf,
    /// Just the filename (e.g., "sodium-1.20.1.jar")
    pub filename: String,
    /// SHA-512 hash of the file (hex-encoded)
    pub sha512: String,
    /// SHA-1 hash of the file (hex-encoded)
    pub sha1: String,
    /// File size in bytes
    pub size: u64,
}

/// Scan a directory for mod files (.jar extension).
///
/// If `recursive` is true, scans subdirectories as well.
/// Computes SHA-512 and SHA-1 hashes for each file found.
pub fn scan_directory(dir: &Path, recursive: bool) -> CoreResult<Vec<LocalMod>> {
    if !dir.exists() {
        return Err(CoreError::Scan(format!(
            "Directory does not exist: {:?}",
            dir
        )));
    }
    if !dir.is_dir() {
        return Err(CoreError::Scan(format!("Not a directory: {:?}", dir)));
    }

    info!("Scanning directory: {:?} (recursive={})", dir, recursive);
    let mut mods = Vec::new();

    let entries = std::fs::read_dir(dir).map_err(|e| {
        CoreError::Scan(format!("Failed to read directory {:?}: {}", dir, e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            CoreError::Scan(format!("Failed to read directory entry: {}", e))
        })?;
        let path = entry.path();

        if path.is_dir() && recursive {
            match scan_directory(&path, recursive) {
                Ok(sub_mods) => mods.extend(sub_mods),
                Err(e) => warn!("Skipping subdirectory {:?}: {}", path, e),
            }
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "jar" {
                    match hash_file(&path) {
                        Ok((sha512, sha1, size)) => {
                            let filename = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            debug!("Found mod: {} (size={}, sha512={:.16}...)", filename, size, sha512);
                            mods.push(LocalMod {
                                path,
                                filename,
                                sha512,
                                sha1,
                                size,
                            });
                        }
                        Err(e) => warn!("Skipping file {:?}: {}", path, e),
                    }
                }
            }
        }
    }

    info!("Scan complete: found {} mod(s)", mods.len());
    Ok(mods)
}

/// Compute SHA-512, SHA-1, and file size for a single file.
/// Uses streaming reads to avoid loading large files entirely into memory.
pub fn hash_file(path: &Path) -> CoreResult<(String, String, u64)> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| {
        CoreError::Scan(format!("Failed to open file {:?}: {}", path, e))
    })?;
    let metadata = file.metadata().map_err(|e| {
        CoreError::Scan(format!("Failed to read metadata for {:?}: {}", path, e))
    })?;
    let size = metadata.len();

    let mut reader = std::io::BufReader::new(file);
    let mut buf = [0u8; 8192];
    let mut hasher512 = Sha512::new();
    let mut hasher1 = sha1::Sha1::new();

    loop {
        let n = reader.read(&mut buf).map_err(|e| {
            CoreError::Scan(format!("Failed to read file {:?}: {}", path, e))
        })?;
        if n == 0 {
            break;
        }
        hasher512.update(&buf[..n]);
        hasher1.update(&buf[..n]);
    }

    let sha512 = hex::encode(hasher512.finalize());
    let sha1 = hex::encode(hasher1.finalize());

    Ok((sha512, sha1, size))
}

/// Compute SHA-512 hash from raw bytes.
pub fn hash_bytes_sha512(data: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compute SHA-1 hash from raw bytes.
pub fn hash_bytes_sha1(data: &[u8]) -> String {
    use sha1::Digest as _;
    let mut hasher = sha1::Sha1::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Parse mod names from a text file, one per line.
/// Lines starting with '#' are treated as comments and skipped.
/// Blank lines are skipped.
pub fn parse_mod_names_from_file(path: &Path) -> CoreResult<Vec<String>> {
    let content =
        std::fs::read_to_string(path).map_err(|e| CoreError::Scan(format!("Failed to read file {:?}: {}", path, e)))?;

    Ok(content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect())
}
