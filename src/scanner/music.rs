use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use std::path::PathBuf;

pub struct MusicJunkScanner {
    search_paths: Vec<(&'static str, PathBuf, SafetyLevel)>,
}

impl MusicJunkScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let search_paths = vec![
            (
                "Music Cache",
                home.join("Library/Caches/com.apple.Music"),
                SafetyLevel::Safe,
            ),
            (
                "Music Streaming Cache",
                home.join("Library/Caches/com.apple.MediaStreaming"),
                SafetyLevel::Safe,
            ),
            (
                "Podcasts Cache",
                home.join("Library/Caches/com.apple.podcasts"),
                SafetyLevel::Safe,
            ),
            (
                "iTunes Cache",
                home.join("Library/Caches/com.apple.iTunes"),
                SafetyLevel::Safe,
            ),
            (
                "Podcasts Downloads",
                home.join(
                    "Library/Group Containers/243LU875E5.groups.com.apple.podcasts/Documents",
                ),
                SafetyLevel::Caution,
            ),
            (
                "Music Library Cache",
                home.join("Music/Music/Media.localized"),
                SafetyLevel::Caution,
            ),
            (
                "iOS Device Backups Cache",
                home.join("Library/Apple/MobileDevice/AllBackupCache"),
                SafetyLevel::Safe,
            ),
            (
                "GarageBand Cache",
                home.join("Library/Application Support/GarageBand"),
                SafetyLevel::Safe,
            ),
            (
                "Logic Cache",
                home.join("Library/Application Support/Logic"),
                SafetyLevel::Safe,
            ),
        ];

        Self { search_paths }
    }
}

impl Scanner for MusicJunkScanner {
    fn id(&self) -> &str {
        "music_junk"
    }

    fn name(&self) -> &str {
        "Music & Podcasts"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for (label, path, safety) in &self.search_paths {
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
                format!("music_{}", items.len()),
                format!("Music - {}", label),
                path.clone(),
            )
            .with_size(size)
            .with_file_count(count_files(path))
            .with_category(ScannerCategory::System)
            .with_safety(*safety)
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
        self.search_paths.iter().any(|(_, p, _)| p.exists())
    }
}

impl Default for MusicJunkScanner {
    fn default() -> Self {
        Self::new()
    }
}
