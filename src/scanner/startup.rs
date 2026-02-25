use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use plist::Value;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

pub struct StartupItemsScanner {
    search_paths: Vec<(String, PathBuf, StartupCategory)>,
}

#[derive(Debug, Clone, Copy)]
pub enum StartupCategory {
    UserLaunchAgent,
    SystemLaunchAgent,
    SystemLaunchDaemon,
    LoginItem,
}

impl StartupCategory {
    fn display_name(&self) -> &'static str {
        match self {
            StartupCategory::UserLaunchAgent => "User LaunchAgent",
            StartupCategory::SystemLaunchAgent => "System LaunchAgent",
            StartupCategory::SystemLaunchDaemon => "System LaunchDaemon",
            StartupCategory::LoginItem => "Login Item",
        }
    }
}

#[derive(Debug, Clone)]
struct StartupItem {
    label: String,
    program: String,
    path: PathBuf,
    category: StartupCategory,
    run_at_load: bool,
    disabled: bool,
}

impl StartupItemsScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let search_paths = vec![
            (
                "User LaunchAgents".to_string(),
                home.join("Library/LaunchAgents"),
                StartupCategory::UserLaunchAgent,
            ),
            (
                "System LaunchAgents".to_string(),
                PathBuf::from("/Library/LaunchAgents"),
                StartupCategory::SystemLaunchAgent,
            ),
            (
                "System LaunchDaemons".to_string(),
                PathBuf::from("/Library/LaunchDaemons"),
                StartupCategory::SystemLaunchDaemon,
            ),
            (
                "Login Items".to_string(),
                home.join("Library/LoginItems"),
                StartupCategory::LoginItem,
            ),
        ];

        Self { search_paths }
    }

    fn parse_plist(path: &PathBuf) -> Option<StartupItem> {
        let content = fs::read(path).ok()?;
        let plist = Value::from_reader(Cursor::new(content)).ok()?;
        let dict = plist.as_dictionary()?;

        let label = dict
            .get("Label")
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
            })
            .to_string();

        let program = dict
            .get("Program")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                dict.get("ProgramArguments")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| path.display().to_string())
            });

        let run_at_load = dict
            .get("RunAtLoad")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        let disabled = dict
            .get("Disabled")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        Some(StartupItem {
            label,
            program,
            path: path.clone(),
            category: StartupCategory::UserLaunchAgent,
            run_at_load,
            disabled,
        })
    }

    fn scan_directory(&self, path: &PathBuf, category: StartupCategory) -> Vec<StartupItem> {
        if !path.exists() {
            return Vec::new();
        }

        let mut items = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path
                    .extension()
                    .map(|e| e == "plist")
                    .unwrap_or(false)
                {
                    if let Some(mut item) = Self::parse_plist(&entry_path) {
                        item.category = category;
                        items.push(item);
                    }
                }
            }
        }

        items
    }
}

impl Scanner for StartupItemsScanner {
    fn id(&self) -> &str {
        "startup_items"
    }

    fn name(&self) -> &str {
        "Startup Items"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for (_, path, category) in &self.search_paths {
            config.report_progress(&path.display().to_string());

            for startup_item in self.scan_directory(path, *category) {
                if config
                    .excluded_paths
                    .iter()
                    .any(|ex| startup_item.path.starts_with(ex))
                {
                    continue;
                }

                let mut item = ScanResult::new(
                    format!("startup_{}", startup_item.label.replace('.', "_")),
                    startup_item.label.clone(),
                    startup_item.path.clone(),
                )
                .with_size(0)
                .with_file_count(1)
                .with_category(ScannerCategory::System)
                .with_safety(SafetyLevel::Caution);

                item.metadata
                    .insert("scanner_id".to_string(), self.id().to_string());
                item.metadata
                    .insert("category".to_string(), category.display_name().to_string());
                item.metadata
                    .insert("program".to_string(), startup_item.program.clone());
                item.metadata.insert(
                    "run_at_load".to_string(),
                    startup_item.run_at_load.to_string(),
                );
                item.metadata
                    .insert("disabled".to_string(), startup_item.disabled.to_string());

                config.report_item(item.clone());
                items.push(item);
            }
        }

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl Default for StartupItemsScanner {
    fn default() -> Self {
        Self::new()
    }
}
