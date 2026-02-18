use anyhow::Result;
use chrono::{DateTime, Utc};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub path: PathBuf,
    pub size: Option<u64>,
}

impl HistoryEntry {
    pub fn new(action: impl Into<String>, path: PathBuf) -> Self {
        Self {
            timestamp: Utc::now(),
            action: action.into(),
            path,
            size: None,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn to_log_line(&self) -> String {
        let size_str = self
            .size
            .map(|s| format!(" size={}", s))
            .unwrap_or_default();
        format!(
            "{} {} {}{}\n",
            self.timestamp.to_rfc3339(),
            self.action,
            self.path.display(),
            size_str
        )
    }
}

pub struct HistoryLogger {
    log_path: PathBuf,
}

impl HistoryLogger {
    pub fn new() -> Self {
        let log_path = Config::data_dir().join("history.log");
        Self { log_path }
    }

    pub fn log(&self, entry: &HistoryEntry) -> Result<()> {
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        write!(file, "{}", entry.to_log_line())?;
        Ok(())
    }

    pub fn log_delete(&self, path: &PathBuf, size: Option<u64>) -> Result<()> {
        let mut entry = HistoryEntry::new("DELETE", path.clone());
        if let Some(s) = size {
            entry = entry.with_size(s);
        }
        self.log(&entry)
    }

    pub fn read_history(&self, limit: Option<usize>) -> Result<Vec<HistoryEntry>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.log_path)?;
        let entries: Vec<HistoryEntry> = content
            .lines()
            .filter_map(|line| self.parse_line(line))
            .collect();

        let result = if let Some(n) = limit {
            entries.into_iter().rev().take(n).collect()
        } else {
            entries
        };

        Ok(result)
    }

    fn parse_line(&self, line: &str) -> Option<HistoryEntry> {
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 3 {
            return None;
        }

        let timestamp = DateTime::parse_from_rfc3339(parts[0])
            .ok()?
            .with_timezone(&Utc);
        let action = parts[1].to_string();
        let path = PathBuf::from(parts[2]);
        let size = parts
            .get(3)
            .and_then(|s| s.strip_prefix("size=").and_then(|s| s.parse::<u64>().ok()));

        Some(HistoryEntry {
            timestamp,
            action,
            path,
            size,
        })
    }

    pub fn clear(&self) -> Result<()> {
        if self.log_path.exists() {
            fs::remove_file(&self.log_path)?;
        }
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.log_path
    }
}

impl Default for HistoryLogger {
    fn default() -> Self {
        Self::new()
    }
}

use crate::config::Config;
