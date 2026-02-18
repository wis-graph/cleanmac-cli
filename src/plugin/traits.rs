use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerCategory {
    System,
    Browser,
    Development,
    Apps,
    Trash,
}

impl std::fmt::Display for ScannerCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScannerCategory::System => write!(f, "System"),
            ScannerCategory::Browser => write!(f, "Browser"),
            ScannerCategory::Development => write!(f, "Development"),
            ScannerCategory::Apps => write!(f, "Apps"),
            ScannerCategory::Trash => write!(f, "Trash"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyLevel {
    Safe,
    Caution,
    Protected,
}

#[derive(Clone)]
pub struct ScanConfig {
    pub min_size: u64,
    pub max_depth: usize,
    pub excluded_paths: Vec<PathBuf>,
    pub follow_symlinks: bool,
    pub progress_callback: Option<Arc<dyn Fn(&str) + Send + Sync>>,
    pub item_callback: Option<Arc<dyn Fn(ScanResult) + Send + Sync>>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            min_size: 1024 * 1024,
            max_depth: 3,
            excluded_paths: Vec::new(),
            follow_symlinks: false,
            progress_callback: None,
            item_callback: None,
        }
    }
}

impl ScanConfig {
    pub fn report_progress(&self, path: &str) {
        if let Some(cb) = &self.progress_callback {
            cb(path);
        }
    }

    pub fn report_item(&self, item: ScanResult) {
        if let Some(cb) = &self.item_callback {
            cb(item);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub last_accessed: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub safety_level: SafetyLevel,
    pub category: ScannerCategory,
    pub metadata: HashMap<String, String>,
}

impl ScanResult {
    pub fn new(id: impl Into<String>, name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            path,
            size: 0,
            file_count: 0,
            dir_count: 0,
            last_accessed: None,
            last_modified: None,
            safety_level: SafetyLevel::Safe,
            category: ScannerCategory::System,
            metadata: HashMap::new(),
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_file_count(mut self, count: u64) -> Self {
        self.file_count = count;
        self
    }

    pub fn with_category(mut self, category: ScannerCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_safety(mut self, level: SafetyLevel) -> Self {
        self.safety_level = level;
        self
    }

    pub fn with_last_accessed(mut self, dt: Option<DateTime<Utc>>) -> Self {
        self.last_accessed = dt;
        self
    }

    pub fn with_last_modified(mut self, dt: Option<DateTime<Utc>>) -> Self {
        self.last_modified = dt;
        self
    }
}

pub trait Scanner: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn category(&self) -> ScannerCategory;
    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>>;
    fn is_available(&self) -> bool {
        true
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(5)
    }
}

#[derive(Debug, Clone)]
pub struct CleanConfig {
    pub dry_run: bool,
    pub log_history: bool,
}

impl Default for CleanConfig {
    fn default() -> Self {
        Self {
            dry_run: true,
            log_history: true,
        }
    }
}

#[derive(Debug)]
pub struct CleanResult {
    pub success_count: usize,
    pub failed_count: usize,
    pub total_freed: u64,
    pub failed_items: Vec<(PathBuf, String)>,
    pub duration: Duration,
}

impl CleanResult {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failed_count: 0,
            total_freed: 0,
            failed_items: Vec::new(),
            duration: Duration::ZERO,
        }
    }
}

impl Default for CleanResult {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Cleaner: Send + Sync {
    fn clean(&self, items: &[ScanResult], config: &CleanConfig) -> Result<CleanResult>;
    fn can_clean(&self, item: &ScanResult) -> bool {
        item.safety_level == SafetyLevel::Safe
    }
}
