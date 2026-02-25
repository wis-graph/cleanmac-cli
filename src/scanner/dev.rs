use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{ScanConfig, ScanResult, Scanner, ScannerCategory};
use crate::safety::SafetyChecker;
use anyhow::Result;
use std::path::PathBuf;

pub struct DevJunkScanner {
    patterns: Vec<(&'static str, &'static str)>,
    search_roots: Vec<PathBuf>,
    safety_checker: SafetyChecker,
}

impl DevJunkScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        Self {
            patterns: vec![
                ("node_modules", "**/node_modules"),
                ("target", "**/target"),
                (".gradle", "**/.gradle"),
                ("build", "**/build"),
                ("dist", "**/dist"),
                (".cache", "**/.cache"),
                ("__pycache__", "**/__pycache__"),
                (".venv", "**/.venv"),
            ],
            search_roots: vec![
                home.join("Documents"),
                home.join("Projects"),
                home.join("Developer"),
                home.join("Workspace"),
                home.join("src"),
                home.join("code"),
            ],
            safety_checker: SafetyChecker::new(),
        }
    }
}

impl Scanner for DevJunkScanner {
    fn id(&self) -> &str {
        "dev_junk"
    }

    fn name(&self) -> &str {
        "Development Junk"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::Development
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for root in &self.search_roots {
            if !root.exists() {
                continue;
            }

            for (pattern_name, pattern) in &self.patterns {
                let full_pattern = root.join(pattern);
                config.report_progress(&full_pattern.to_string_lossy());

                for entry in glob::glob(&full_pattern.to_string_lossy())?.filter_map(|e| e.ok()) {
                    if !entry.is_dir() {
                        continue;
                    }

                    if config.excluded_paths.iter().any(|ex| entry.starts_with(ex)) {
                        continue;
                    }

                    let size = calculate_dir_size(&entry);

                    if size >= config.min_size {
                        let name = entry
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let safety_level = self.safety_checker.check_path(&entry);

                        let mut item = ScanResult::new(
                            format!("dev_{}_{}", pattern_name, items.len()),
                            format!("{} ({})", name, pattern_name),
                            entry.clone(),
                        )
                        .with_size(size)
                        .with_file_count(count_files(&entry))
                        .with_category(ScannerCategory::Development)
                        .with_safety(safety_level)
                        .with_last_accessed(get_last_accessed(&entry))
                        .with_last_modified(get_last_modified(&entry));

                        item.metadata
                            .insert("scanner_id".to_string(), self.id().to_string());

                        config.report_item(item.clone());
                        items.push(item);
                    }
                }
            }
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        items.truncate(50);

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for DevJunkScanner {
    fn default() -> Self {
        Self::new()
    }
}
