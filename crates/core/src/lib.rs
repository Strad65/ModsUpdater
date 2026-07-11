pub mod api;
pub mod config;
pub mod error;
pub mod scanner;
pub mod updater;

// Re-export key types for convenience
pub use api::ModrinthClient;
pub use config::AppConfig;
pub use error::{CoreError, CoreResult};
pub use scanner::{hash_bytes_sha1, hash_bytes_sha512, hash_file, parse_mod_names_from_file, scan_directory, LocalMod};
pub use updater::{analyze_mods, install_update, load_latest_report, run_updates, save_report, ModInfo, UpdateReport, UpdateResult};
