use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use crate::safety::SafetyChecker;
use anyhow::Result;
use std::path::PathBuf;

pub struct TrashScanner {
    trash_paths: Vec<PathBuf>,
    safety_checker: SafetyChecker,
}

impl TrashScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        Self {
            trash_paths: vec![home.join(".Trash")],
            safety_checker: SafetyChecker::new(),
        }
    }
}

impl Scanner for TrashScanner {
    fn id(&self) -> &str {
        "trash"
    }

    fn name(&self) -> &str {
        "Trash"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::Trash
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for trash_path in &self.trash_paths {
            if !trash_path.exists() {
                continue;
            }

            let size = calculate_dir_size(trash_path);

            if size > 0 {
                let file_count = count_files(trash_path);

                items.push(
                    ScanResult::new("trash_main", "Trash", trash_path.clone())
                        .with_size(size)
                        .with_file_count(file_count)
                        .with_category(ScannerCategory::Trash)
                        .with_safety(SafetyLevel::Safe)
                        .with_last_accessed(get_last_accessed(trash_path))
                        .with_last_modified(get_last_modified(trash_path)),
                );
            }
        }

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for TrashScanner {
    fn default() -> Self {
        Self::new()
    }
}
