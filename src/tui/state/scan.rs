use crate::plugin::{ScanResult, ScannerCategory};
use std::time::Duration;

pub enum ScanMessage {
    ScannerStart {
        name: String,
    },
    ScanningPath {
        path: String,
    },
    ItemFound {
        scanner_id: String,
        item: ScanResult,
    },
    ScannerDone {
        scanner_id: String,
        name: String,
        category: ScannerCategory,
    },
    ScanComplete,
}

pub struct ScannerInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub current_scanner: String,
    pub current_path: Option<String>,
    pub scanners_done: usize,
    pub total_scanners: usize,
    pub active_scanners: usize,
}

#[derive(Debug, Clone)]
pub struct CleanResultDisplay {
    pub success_count: usize,
    pub failed_count: usize,
    pub total_freed: u64,
    pub duration: Duration,
}
