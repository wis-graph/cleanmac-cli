use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use std::path::PathBuf;

pub struct PhotoJunkScanner {
    search_paths: Vec<(&'static str, PathBuf)>,
}

impl PhotoJunkScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let photos_lib = home.join("Pictures/Photos Library.photoslibrary");

        let search_paths = vec![
            (
                "Thumbnails",
                photos_lib.join("resources/derivatives/thumbs"),
            ),
            ("Caches", photos_lib.join("resources/caches")),
            (
                "Compute Cache",
                photos_lib.join("private/com.apple.photolibraryd/caches/computecache"),
            ),
            (
                "Analysis Cache",
                photos_lib.join("private/com.apple.photoanalysisd/caches"),
            ),
            (
                "iCloud Sync Cache",
                photos_lib.join("resources/cpl/cloudsync.noindex"),
            ),
            ("Spotlight Cache", photos_lib.join("database/search")),
        ];

        Self { search_paths }
    }
}

impl Scanner for PhotoJunkScanner {
    fn id(&self) -> &str {
        "photo_junk"
    }

    fn name(&self) -> &str {
        "Photo Junk"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for (label, path) in &self.search_paths {
            if !path.exists() {
                continue;
            }

            config.report_progress(&path.display().to_string());

            if config.excluded_paths.iter().any(|ex| path.starts_with(ex)) {
                continue;
            }

            let size = calculate_dir_size(path);
            if size < config.min_size {
                continue;
            }

            let mut item = ScanResult::new(
                format!("photo_{}", items.len()),
                format!("Photos - {}", label),
                path.clone(),
            )
            .with_size(size)
            .with_file_count(count_files(path))
            .with_category(ScannerCategory::System)
            .with_safety(SafetyLevel::Caution)
            .with_last_accessed(get_last_accessed(path))
            .with_last_modified(get_last_modified(path));

            item.metadata
                .insert("scanner_id".to_string(), self.id().to_string());

            config.report_item(item.clone());
            items.push(item);
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        Ok(items)
    }

    fn is_available(&self) -> bool {
        self.search_paths.iter().any(|(_, p)| p.exists())
    }
}

impl Default for PhotoJunkScanner {
    fn default() -> Self {
        Self::new()
    }
}
