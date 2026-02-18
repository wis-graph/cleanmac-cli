use super::traits::{CleanConfig, CleanResult, Cleaner, ScanConfig, ScanResult, Scanner};
use crate::scanner::{BrowserCacheScanner, CacheScanner, DevJunkScanner, LogScanner, TrashScanner};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

pub struct PluginRegistry {
    scanners: Vec<Box<dyn Scanner>>,
    cleaners: Vec<Box<dyn Cleaner>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            scanners: Vec::new(),
            cleaners: Vec::new(),
        }
    }

    pub fn register_scanner(&mut self, scanner: Box<dyn Scanner>) {
        self.scanners.push(scanner);
    }

    pub fn register_cleaner(&mut self, cleaner: Box<dyn Cleaner>) {
        self.cleaners.push(cleaner);
    }

    pub fn scanners(&self) -> &[Box<dyn Scanner>] {
        &self.scanners
    }

    pub fn scan_all(&self, config: &ScanConfig) -> Result<ScanReport> {
        let start = Instant::now();

        let category_results: Vec<_> = self
            .scanners
            .par_iter()
            .filter(|s| s.is_available())
            .map(|scanner| {
                let results = scanner.scan(config).unwrap_or_default();
                CategoryScanResult {
                    scanner_id: scanner.id().to_string(),
                    name: scanner.name().to_string(),
                    category: scanner.category(),
                    icon: scanner.icon().to_string(),
                    items: results,
                }
            })
            .collect();

        let total_size: u64 = category_results
            .iter()
            .flat_map(|c| c.items.iter())
            .map(|i| i.size)
            .sum();
        let total_items: usize = category_results.iter().map(|c| c.items.len()).sum();

        Ok(ScanReport {
            categories: category_results,
            total_size,
            total_items,
            duration: start.elapsed(),
        })
    }

    pub fn clean(&self, items: &[ScanResult], config: &CleanConfig) -> Result<CleanResult> {
        if let Some(cleaner) = self.cleaners.first() {
            cleaner.clean(items, config)
        } else {
            Ok(CleanResult::new())
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        registry.register_scanner(Box::new(CacheScanner::new()));
        registry.register_scanner(Box::new(LogScanner::new()));
        registry.register_scanner(Box::new(TrashScanner::new()));
        registry.register_scanner(Box::new(BrowserCacheScanner::new()));
        registry.register_scanner(Box::new(DevJunkScanner::new()));

        registry
    }
}

#[derive(Debug, Clone)]
pub struct CategoryScanResult {
    pub scanner_id: String,
    pub name: String,
    pub category: super::traits::ScannerCategory,
    pub icon: String,
    pub items: Vec<ScanResult>,
}

impl CategoryScanResult {
    pub fn total_size(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }

    pub fn total_files(&self) -> u64 {
        self.items.iter().map(|i| i.file_count).sum()
    }
}

#[derive(Debug)]
pub struct ScanReport {
    pub categories: Vec<CategoryScanResult>,
    pub total_size: u64,
    pub total_items: usize,
    pub duration: std::time::Duration,
}

impl ScanReport {
    pub fn is_empty(&self) -> bool {
        self.categories.is_empty() || self.total_items == 0
    }
}
