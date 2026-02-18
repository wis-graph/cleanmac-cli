pub mod browser;
pub mod caches;
pub mod dev;
pub mod large_files;
pub mod logs;
pub mod mail;
pub mod trash;

pub use browser::BrowserCacheScanner;
pub use caches::CacheScanner;
pub use dev::DevJunkScanner;
pub use large_files::LargeOldFilesScanner;
pub use logs::LogScanner;
pub use mail::MailAttachmentsScanner;
pub use trash::TrashScanner;

use chrono::{DateTime, Utc};
use std::path::Path;
use walkdir::WalkDir;

fn calculate_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

fn count_files(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u64
}

fn get_last_accessed(path: &Path) -> Option<DateTime<Utc>> {
    path.metadata()
        .ok()
        .and_then(|m| m.accessed().ok())
        .map(|t| t.into())
}

fn get_last_modified(path: &Path) -> Option<DateTime<Utc>> {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| t.into())
}
