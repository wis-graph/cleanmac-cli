use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub categories: Vec<CategoryScanResult>,
    pub total_size_bytes: u64,
    pub total_item_count: usize,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScanResult {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub item_count: usize,
    pub items: Vec<ScanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanItem {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub scan_file: Option<String>,
    pub categories: Vec<CategoryPlanResult>,
    pub total_size_bytes: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPlanResult {
    pub id: String,
    pub action: String,
    pub items: Vec<PlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub plan_file: Option<String>,
    pub status: ExecutionStatus,
    pub categories: Vec<CategoryExecutionResult>,
    pub total_deleted_size: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryExecutionResult {
    pub id: String,
    pub status: ExecutionStatus,
    pub deleted_count: usize,
    pub deleted_size_bytes: u64,
    pub failed_count: usize,
    pub failed_items: Vec<FailedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedItem {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Partial,
    Failed,
    Cancelled,
}

impl ScanResult {
    pub fn new(categories: Vec<CategoryScanResult>, duration_ms: u64) -> Self {
        let total_size_bytes = categories.iter().map(|c| c.size_bytes).sum();
        let total_item_count = categories.iter().map(|c| c.item_count).sum();

        Self {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            categories,
            total_size_bytes,
            total_item_count,
            scan_duration_ms: duration_ms,
        }
    }
}

impl PlanResult {
    pub fn new(categories: Vec<CategoryPlanResult>, scan_file: Option<String>) -> Self {
        let total_size_bytes = categories
            .iter()
            .map(|c| c.items.iter().map(|i| i.size_bytes).sum::<u64>())
            .sum();

        Self {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            scan_file,
            categories,
            total_size_bytes,
            warnings: Vec::new(),
        }
    }
}

impl ExecutionResult {
    pub fn new(
        plan_file: Option<String>,
        categories: Vec<CategoryExecutionResult>,
        duration_ms: u64,
    ) -> Self {
        let total_deleted_size = categories.iter().map(|c| c.deleted_size_bytes).sum();
        let status = if categories
            .iter()
            .all(|c| c.status == ExecutionStatus::Success)
        {
            ExecutionStatus::Success
        } else if categories
            .iter()
            .any(|c| c.status == ExecutionStatus::Success)
        {
            ExecutionStatus::Partial
        } else {
            ExecutionStatus::Failed
        };

        Self {
            version: "1.0".to_string(),
            timestamp: Utc::now(),
            plan_file,
            status,
            categories,
            total_deleted_size,
            duration_ms,
        }
    }
}
