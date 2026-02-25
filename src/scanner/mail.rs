use super::{calculate_dir_size, count_files, get_last_accessed, get_last_modified};
use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use std::path::PathBuf;

pub struct MailAttachmentsScanner {
    search_paths: Vec<(&'static str, PathBuf)>,
}

impl MailAttachmentsScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let search_paths = vec![
            ("Mail Attachments", home.join("Library/Mail")),
            (
                "Mail Downloads",
                home.join("Library/Containers/com.apple.mail/Data/Library/Mail Downloads"),
            ),
        ];

        Self { search_paths }
    }

    fn find_attachment_dirs(&self, base: &PathBuf) -> Vec<PathBuf> {
        let mut results = Vec::new();

        use walkdir::WalkDir;
        for entry in WalkDir::new(base)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "Attachments" || name == "Mail Downloads" {
                    results.push(path.to_path_buf());
                }
            }
        }

        results
    }
}

impl Scanner for MailAttachmentsScanner {
    fn id(&self) -> &str {
        "mail_attachments"
    }

    fn name(&self) -> &str {
        "Mail Attachments"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for (label, base_path) in &self.search_paths {
            if !base_path.exists() {
                continue;
            }

            config.report_progress(&base_path.display().to_string());

            let attachment_dirs = self.find_attachment_dirs(base_path);

            for dir in attachment_dirs {
                if config.excluded_paths.iter().any(|ex| dir.starts_with(ex)) {
                    continue;
                }

                let size = calculate_dir_size(&dir);
                if size < config.min_size {
                    continue;
                }

                let name = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Attachments")
                    .to_string();

                let parent_name = dir
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                let display_name = if parent_name.contains('@') {
                    format!("{} ({})", name, parent_name)
                } else {
                    format!("{} ({})", name, label)
                };

                let mut item =
                    ScanResult::new(format!("mail_{}", items.len()), display_name, dir.clone())
                        .with_size(size)
                        .with_file_count(count_files(&dir))
                        .with_category(ScannerCategory::System)
                        .with_safety(SafetyLevel::Caution)
                        .with_last_accessed(get_last_accessed(&dir))
                        .with_last_modified(get_last_modified(&dir));

                item.metadata
                    .insert("scanner_id".to_string(), self.id().to_string());

                config.report_item(item.clone());
                items.push(item);
            }
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        Ok(items)
    }

    fn is_available(&self) -> bool {
        self.search_paths.iter().any(|(_, p)| p.exists())
    }
}

impl Default for MailAttachmentsScanner {
    fn default() -> Self {
        Self::new()
    }
}
