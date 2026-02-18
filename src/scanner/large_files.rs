use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::WalkDir;

const DEFAULT_MIN_SIZE: u64 = 100 * 1024 * 1024; // 100MB
const DEFAULT_MIN_AGE_DAYS: i64 = 30;

pub struct LargeOldFilesScanner {
    home: PathBuf,
    excluded_dirs: Vec<PathBuf>,
}

impl LargeOldFilesScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let excluded_dirs = vec![
            home.join("Library"),
            home.join(".Trash"),
            home.join("Applications"),
            home.join("Music"),
            home.join("Movies"),
            home.join("Pictures"),
            home.join(".config"),
            home.join(".cache"),
        ];

        Self {
            home,
            excluded_dirs,
        }
    }

    fn is_excluded(&self, path: &std::path::Path) -> bool {
        for excluded in &self.excluded_dirs {
            if path.starts_with(excluded) {
                return true;
            }
        }
        false
    }

    fn get_file_age_days(path: &std::path::Path) -> Option<i64> {
        let metadata = path.metadata().ok()?;
        let accessed = metadata.accessed().ok()?;
        let modified = metadata.modified().ok()?;

        let older_time = if accessed < modified {
            accessed
        } else {
            modified
        };
        let now = SystemTime::now();
        let duration = now.duration_since(older_time).ok()?;

        Some(duration.as_secs() as i64 / 86400)
    }
}

impl Scanner for LargeOldFilesScanner {
    fn id(&self) -> &str {
        "large_old_files"
    }

    fn name(&self) -> &str {
        "Large & Old Files"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();
        let min_size = if config.min_size > 0 {
            config.min_size
        } else {
            DEFAULT_MIN_SIZE
        };
        let cutoff_days = DEFAULT_MIN_AGE_DAYS;

        let max_depth = if config.max_depth > 0 {
            config.max_depth
        } else {
            10
        };

        for entry in WalkDir::new(&self.home)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                if self.is_excluded(path) {
                    return false;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') && path.is_dir() {
                        return false;
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = metadata.len();
            if size < min_size {
                continue;
            }

            let age_days = match Self::get_file_age_days(path) {
                Some(days) => days,
                None => continue,
            };

            if age_days < cutoff_days {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();

            let last_accessed = metadata.accessed().ok().map(|t| t.into());

            let last_modified = metadata.modified().ok().map(|t| t.into());

            items.push(
                ScanResult::new(
                    format!("large_file_{}", items.len()),
                    file_name,
                    path.to_path_buf(),
                )
                .with_size(size)
                .with_file_count(1)
                .with_category(ScannerCategory::System)
                .with_safety(SafetyLevel::Caution)
                .with_last_accessed(last_accessed)
                .with_last_modified(last_modified),
            );
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        items.truncate(100);

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for LargeOldFilesScanner {
    fn default() -> Self {
        Self::new()
    }
}
