use crate::plugin::SafetyLevel;
use anyhow::Result;
use std::path::Path;

pub struct SafetyChecker {
    protected_paths: Vec<&'static str>,
    critical_patterns: Vec<&'static str>,
}

impl SafetyChecker {
    pub fn new() -> Self {
        Self {
            protected_paths: vec![
                "/System",
                "/usr",
                "/bin",
                "/sbin",
                "/etc",
                "/var/db",
                "/private/var/db",
            ],
            critical_patterns: vec![
                ".Spotlight-",
                ".fseventsd",
                ".Trashes",
                "Library/Keychains",
                "Library/Security",
                "Library/CoreServices",
            ],
        }
    }

    pub fn check_path(&self, path: &Path) -> SafetyLevel {
        let path_str = path.to_string_lossy();

        for protected in &self.protected_paths {
            if path_str.starts_with(protected) {
                return SafetyLevel::Protected;
            }
        }

        for pattern in &self.critical_patterns {
            if path_str.contains(pattern) {
                return SafetyLevel::Protected;
            }
        }

        if self.is_hidden_system(path) {
            return SafetyLevel::Caution;
        }

        SafetyLevel::Safe
    }

    fn is_hidden_system(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.') && !n.starts_with(".."))
            .unwrap_or(false)
    }

    pub fn is_safe_to_delete(&self, path: &Path) -> bool {
        matches!(
            self.check_path(path),
            SafetyLevel::Safe | SafetyLevel::Caution
        )
    }

    pub fn get_running_apps(&self) -> Result<Vec<String>> {
        use std::process::Command;

        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get name of every process whose background only is false")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let apps = String::from_utf8_lossy(&output.stdout);
                Ok(apps.split(", ").map(|s| s.trim().to_string()).collect())
            }
            _ => Ok(Vec::new()),
        }
    }

    pub fn is_app_running(&self, app_name: &str) -> bool {
        self.get_running_apps()
            .map(|apps| {
                apps.iter()
                    .any(|a| a.to_lowercase().contains(&app_name.to_lowercase()))
            })
            .unwrap_or(false)
    }
}

impl Default for SafetyChecker {
    fn default() -> Self {
        Self::new()
    }
}
