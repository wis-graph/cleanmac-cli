use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use std::path::PathBuf;

pub struct MaintenanceScanner {
    tasks: Vec<MaintenanceTask>,
}

struct MaintenanceTask {
    id: String,
    name: String,
    description: String,
    command: String,
    requires_sudo: bool,
    safety: SafetyLevel,
}

impl MaintenanceScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let tasks = vec![
            MaintenanceTask {
                id: "flush_dns".into(),
                name: "Flush DNS Cache".into(),
                description: "Clear DNS cache to resolve network issues".into(),
                command: "dscacheutil -flushcache && sudo killall -HUP mDNSResponder".into(),
                requires_sudo: true,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "rebuild_launchservices".into(),
                name: "Rebuild Launch Services".into(),
                description: "Rebuild Launch Services database to fix app associations".into(),
                command: "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user".into(),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "clear_font_cache".into(),
                name: "Clear Font Cache".into(),
                description: "Clear font cache to fix font rendering issues".into(),
                command: "atsutil databases -remove".into(),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "reset_spotlight".into(),
                name: "Reset Spotlight Index".into(),
                description: "Reset Spotlight search index (may take time)".into(),
                command: "sudo mdutil -E /".into(),
                requires_sudo: true,
                safety: SafetyLevel::Caution,
            },
            MaintenanceTask {
                id: "purge_memory".into(),
                name: "Purge Memory".into(),
                description: "Free up inactive memory".into(),
                command: "purge".into(),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "clean_tmp".into(),
                name: "Clean TMP Files".into(),
                description: "Remove temporary system files".into(),
                command: format!("rm -rf /tmp/* 2>/dev/null; rm -rf {}/.tmp/* 2>/dev/null", home.display()),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "verify_disk".into(),
                name: "Verify Disk".into(),
                description: "Verify startup disk for errors".into(),
                command: "diskutil verifyVolume /".into(),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "clear_quicklook".into(),
                name: "Clear Quick Look Cache".into(),
                description: "Clear Quick Look thumbnail cache".into(),
                command: "qlmanage -r cache".into(),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
            MaintenanceTask {
                id: "reset_dock".into(),
                name: "Reset Dock".into(),
                description: "Reset Dock to default settings".into(),
                command: "defaults delete com.apple.dock; killall Dock".into(),
                requires_sudo: false,
                safety: SafetyLevel::Caution,
            },
            MaintenanceTask {
                id: "reset_finder".into(),
                name: "Reset Finder".into(),
                description: "Restart Finder to apply changes".into(),
                command: "killall Finder".into(),
                requires_sudo: false,
                safety: SafetyLevel::Safe,
            },
        ];

        Self { tasks }
    }
}

impl Scanner for MaintenanceScanner {
    fn id(&self) -> &str {
        "maintenance"
    }

    fn name(&self) -> &str {
        "Maintenance"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::System
    }

    fn icon(&self) -> &str {
        ""
    }

    fn scan(&self, _config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for task in &self.tasks {
            let mut item = ScanResult::new(
                format!("maint_{}", task.id),
                task.name.clone(),
                PathBuf::from(&task.command),
            )
            .with_size(0)
            .with_file_count(1)
            .with_category(ScannerCategory::System)
            .with_safety(task.safety);

            item.metadata
                .insert("scanner_id".to_string(), self.id().to_string());
            item.metadata.insert("task_id".to_string(), task.id.clone());
            item.metadata
                .insert("command".to_string(), task.command.clone());
            item.metadata
                .insert("description".to_string(), task.description.clone());
            item.metadata
                .insert("requires_sudo".to_string(), task.requires_sudo.to_string());

            _config.report_item(item.clone());
            items.push(item);
        }

        Ok(items)
    }

    fn is_available(&self) -> bool {
        true
    }

    fn estimated_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(1)
    }
}

impl Default for MaintenanceScanner {
    fn default() -> Self {
        Self::new()
    }
}
