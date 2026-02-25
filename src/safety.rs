use crate::plugin::SafetyLevel;
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
}

impl Default for SafetyChecker {
    fn default() -> Self {
        Self::new()
    }
}
