use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use crate::safety::SafetyChecker;
use anyhow::Result;
use std::path::PathBuf;

pub struct BrowserCacheScanner {
    cache_paths: Vec<(String, PathBuf)>,
    safety_checker: SafetyChecker,
}

impl BrowserCacheScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        Self {
            cache_paths: vec![
                (
                    "Safari".to_string(),
                    home.join("Library/Caches/com.apple.Safari"),
                ),
                (
                    "Chrome".to_string(),
                    home.join("Library/Caches/Google/Chrome"),
                ),
                ("Firefox".to_string(), home.join("Library/Caches/Firefox")),
                (
                    "Edge".to_string(),
                    home.join("Library/Caches/Microsoft Edge"),
                ),
                ("Arc".to_string(), home.join("Library/Caches/Arc")),
                (
                    "Brave".to_string(),
                    home.join("Library/Caches/BraveSoftware"),
                ),
                ("Vivaldi".to_string(), home.join("Library/Caches/Vivaldi")),
                (
                    "Opera".to_string(),
                    home.join("Library/Caches/com.operasoftware.Opera"),
                ),
                (
                    "Opera GX".to_string(),
                    home.join("Library/Caches/com.operasoftware.OperaGX"),
                ),
                ("Comet".to_string(), home.join("Library/Caches/Comet")),
                ("Whale".to_string(), home.join("Library/Caches/Naver/Whale")),
                ("Chromium".to_string(), home.join("Library/Caches/Chromium")),
                (
                    "Orion".to_string(),
                    home.join("Library/Caches/com.kagi.kagimac"),
                ),
            ],
            safety_checker: SafetyChecker::new(),
        }
    }
}

impl Scanner for BrowserCacheScanner {
    fn id(&self) -> &str {
        "browser_cache"
    }

    fn name(&self) -> &str {
        "Browser Caches"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::Browser
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for (browser_name, cache_path) in &self.cache_paths {
            if !cache_path.exists() {
                continue;
            }

            let size = calculate_dir_size(cache_path);

            if size >= config.min_size {
                let file_count = count_files(cache_path);

                items.push(
                    ScanResult::new(
                        format!("browser_{}", browser_name.to_lowercase()),
                        format!("{} Cache", browser_name),
                        cache_path.clone(),
                    )
                    .with_size(size)
                    .with_file_count(file_count)
                    .with_category(ScannerCategory::Browser)
                    .with_safety(SafetyLevel::Safe)
                    .with_last_accessed(get_last_accessed(cache_path))
                    .with_last_modified(get_last_modified(cache_path)),
                );
            }
        }

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for BrowserCacheScanner {
    fn default() -> Self {
        Self::new()
    }
}
