use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{ScanConfig, ScanResult, Scanner, ScannerCategory};
use crate::safety::SafetyChecker;
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct LogScanner {
    log_dirs: Vec<PathBuf>,
    safety_checker: SafetyChecker,
}

impl LogScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        Self {
            log_dirs: vec![home.join("Library/Logs")],
            safety_checker: SafetyChecker::new(),
        }
    }
}

impl Scanner for LogScanner {
    fn id(&self) -> &str {
        "system_logs"
    }

    fn name(&self) -> &str {
        "System Logs"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for log_dir in &self.log_dirs {
            if !log_dir.exists() {
                continue;
            }

            for entry in WalkDir::new(log_dir)
                .max_depth(config.max_depth)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                if config.excluded_paths.iter().any(|ex| path.starts_with(ex)) {
                    continue;
                }

                let (size, file_count) = if entry.file_type().is_dir() {
                    (calculate_dir_size(path), count_files(path))
                } else if entry.file_type().is_file() {
                    let metadata = entry.metadata()?;
                    (metadata.len(), 1)
                } else {
                    continue;
                };

                if size >= config.min_size {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let safety_level = self.safety_checker.check_path(path);

                    items.push(
                        ScanResult::new(format!("log_{}", items.len()), name, path.to_path_buf())
                            .with_size(size)
                            .with_file_count(file_count)
                            .with_category(ScannerCategory::System)
                            .with_safety(safety_level)
                            .with_last_accessed(get_last_accessed(path))
                            .with_last_modified(get_last_modified(path)),
                    );
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

impl Default for LogScanner {
    fn default() -> Self {
        Self::new()
    }
}
