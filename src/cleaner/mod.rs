use crate::history::HistoryLogger;
use crate::plugin::{CleanConfig, CleanResult, Cleaner, SafetyLevel, ScanResult};
use crate::safety::SafetyChecker;
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

pub struct DefaultCleaner {
    safety_checker: SafetyChecker,
    history_logger: HistoryLogger,
}

impl DefaultCleaner {
    pub fn new() -> Self {
        Self {
            safety_checker: SafetyChecker::new(),
            history_logger: HistoryLogger::new(),
        }
    }
}

impl Cleaner for DefaultCleaner {
    fn clean(&self, items: &[ScanResult], config: &CleanConfig) -> Result<CleanResult> {
        let start = Instant::now();
        let mut result = CleanResult::new();

        for item in items {
            if !self.can_clean(item) {
                result
                    .failed_items
                    .push((item.path.clone(), "Not safe to delete".to_string()));
                result.failed_count += 1;
                continue;
            }

            match self.delete_path(&item.path, config.dry_run) {
                Ok(()) => {
                    result.success_count += 1;
                    result.total_freed += item.size;

                    if config.log_history {
                        let _ = self.history_logger.log_delete(&item.path, Some(item.size));
                    }
                }
                Err(e) => {
                    result.failed_items.push((item.path.clone(), e.to_string()));
                    result.failed_count += 1;
                }
            }
        }

        result.duration = start.elapsed();
        Ok(result)
    }

    fn can_clean(&self, item: &ScanResult) -> bool {
        matches!(item.safety_level, SafetyLevel::Safe | SafetyLevel::Caution)
            && self.safety_checker.is_safe_to_delete(&item.path)
    }
}

impl DefaultCleaner {
    fn delete_path(&self, path: &Path, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("[DRY-RUN] Would delete: {}", path.display());
            return Ok(());
        }

        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }

        println!("Deleted: {}", path.display());
        Ok(())
    }
}

impl Default for DefaultCleaner {
    fn default() -> Self {
        Self::new()
    }
}
