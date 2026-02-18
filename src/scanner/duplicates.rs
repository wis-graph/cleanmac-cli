use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

const MIN_SIZE: u64 = 1024;

pub struct DuplicatesScanner {
    search_paths: Vec<PathBuf>,
}

impl DuplicatesScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let search_paths = vec![
            home.join("Documents"),
            home.join("Downloads"),
            home.join("Desktop"),
            home.join("Pictures"),
            home.join("Movies"),
            home.join("Music"),
        ];

        Self { search_paths }
    }

    fn calculate_file_hash(path: &std::path::Path) -> Result<String> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn get_file_modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
        path.metadata().ok().and_then(|m| m.modified().ok())
    }
}

impl Scanner for DuplicatesScanner {
    fn id(&self) -> &str {
        "duplicates"
    }

    fn name(&self) -> &str {
        "Duplicates"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();
        let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();

        for root in &self.search_paths {
            if !root.exists() {
                continue;
            }

            for entry in WalkDir::new(root)
                .max_depth(config.max_depth)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();

                if config.excluded_paths.iter().any(|ex| path.starts_with(ex)) {
                    continue;
                }

                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }

                config.report_progress(&path.display().to_string());

                if let Ok(metadata) = path.metadata() {
                    let size = metadata.len();
                    if size >= MIN_SIZE.max(config.min_size) {
                        size_map.entry(size).or_default().push(path.to_path_buf());
                    }
                }
            }
        }

        let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for (size, paths) in size_map {
            if paths.len() < 2 {
                continue;
            }

            for path in paths {
                if let Ok(hash) = Self::calculate_file_hash(&path) {
                    let key = format!("{}:{}", size, hash);
                    hash_map.entry(key).or_default().push(path);
                }
            }
        }

        let mut group_id = 0;
        for (_key, mut paths) in hash_map {
            if paths.len() < 2 {
                continue;
            }

            paths.sort_by(|a, b| {
                let a_time =
                    Self::get_file_modified(a).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let b_time =
                    Self::get_file_modified(b).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                a_time.cmp(&b_time)
            });

            let original = &paths[0];
            let duplicates = &paths[1..];

            if let Ok(metadata) = original.metadata() {
                let total_dup_size: u64 = duplicates
                    .iter()
                    .filter_map(|p| p.metadata().ok())
                    .map(|m| m.len())
                    .sum();

                let mut item = ScanResult::new(
                    format!("dup_{}", group_id),
                    format!(
                        "{} ({} duplicates)",
                        original.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                        duplicates.len()
                    ),
                    original.clone(),
                )
                .with_size(total_dup_size)
                .with_file_count(duplicates.len() as u64)
                .with_category(ScannerCategory::System)
                .with_safety(SafetyLevel::Caution)
                .with_last_accessed(metadata.accessed().ok().map(|t| chrono::DateTime::from(t)))
                .with_last_modified(metadata.modified().ok().map(|t| chrono::DateTime::from(t)));

                item.metadata
                    .insert("scanner_id".to_string(), self.id().to_string());
                item.metadata
                    .insert("group_id".to_string(), group_id.to_string());
                item.metadata.insert(
                    "duplicate_paths".to_string(),
                    duplicates
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("|"),
                );
                item.metadata
                    .insert("original_path".to_string(), original.display().to_string());

                config.report_item(item.clone());
                items.push(item);
            }

            group_id += 1;
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        Ok(items)
    }

    fn is_available(&self) -> bool {
        self.search_paths.iter().any(|p| p.exists())
    }

    fn estimated_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
}

impl Default for DuplicatesScanner {
    fn default() -> Self {
        Self::new()
    }
}
