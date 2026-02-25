use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{ScanConfig, ScanResult, Scanner, ScannerCategory};
use crate::safety::SafetyChecker;
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct CacheScanner {
    cache_dirs: Vec<PathBuf>,
    safety_checker: SafetyChecker,
}

impl CacheScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        Self {
            cache_dirs: vec![
                home.join("Library/Caches"),
                home.join("Library/Developer/Xcode/DerivedData"),
            ],
            safety_checker: SafetyChecker::new(),
        }
    }
}

impl Scanner for CacheScanner {
    fn id(&self) -> &str {
        "system_caches"
    }

    fn name(&self) -> &str {
        "System Caches"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for cache_dir in &self.cache_dirs {
            if !cache_dir.exists() {
                continue;
            }

            for entry in WalkDir::new(cache_dir)
                .max_depth(config.max_depth)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
            {
                let path = entry.path();

                if config.excluded_paths.iter().any(|ex| path.starts_with(ex)) {
                    continue;
                }

                config.report_progress(&path.display().to_string());

                let size = calculate_dir_size(path);

                if size >= config.min_size {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let safety_level = self.safety_checker.check_path(path);

                    let mut item =
                        ScanResult::new(format!("cache_{}", items.len()), name, path.to_path_buf())
                            .with_size(size)
                            .with_file_count(count_files(path))
                            .with_category(ScannerCategory::System)
                            .with_safety(safety_level)
                            .with_last_accessed(get_last_accessed(path))
                            .with_last_modified(get_last_modified(path));

                    item.metadata
                        .insert("scanner_id".to_string(), self.id().to_string());

                    config.report_item(item.clone());
                    items.push(item);
                }
            }
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        items.truncate(100);

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for CacheScanner {
    fn default() -> Self {
        Self::new()
    }
}
