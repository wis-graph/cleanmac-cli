use crate::plugin::{SafetyLevel, ScanConfig, ScanResult, Scanner, ScannerCategory};
use anyhow::Result;
use std::path::PathBuf;

pub struct PrivacyScanner {
    search_paths: Vec<(&'static str, PathBuf, SafetyLevel)>,
}

impl PrivacyScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let search_paths = vec![
            ("Safari Cookies", home.join("Library/Cookies/com.apple.Safari/ Cookies.binarycookies"), SafetyLevel::Caution),
            ("Safari History", home.join("Library/Safari/History.db"), SafetyLevel::Caution),
            ("Safari Downloads", home.join("Library/Safari/Downloads.plist"), SafetyLevel::Caution),
            
            ("Chrome Cookies", home.join("Library/Application Support/Google/Chrome/Default/Cookies"), SafetyLevel::Caution),
            ("Chrome History", home.join("Library/Application Support/Google/Chrome/Default/History"), SafetyLevel::Caution),
            ("Chrome Login Data", home.join("Library/Application Support/Google/Chrome/Default/Login Data"), SafetyLevel::Protected),
            
            ("Firefox Cookies", home.join("Library/Application Support/Firefox/Profiles/cookies.sqlite"), SafetyLevel::Caution),
            ("Firefox History", home.join("Library/Application Support/Firefox/Profiles/places.sqlite"), SafetyLevel::Caution),
            
            ("Edge Cookies", home.join("Library/Application Support/Microsoft Edge/Default/Cookies"), SafetyLevel::Caution),
            ("Edge History", home.join("Library/Application Support/Microsoft Edge/Default/History"), SafetyLevel::Caution),
            
            ("Brave Cookies", home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default/Cookies"), SafetyLevel::Caution),
            ("Brave History", home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default/History"), SafetyLevel::Caution),
            
            ("Arc Cookies", home.join("Library/Application Support/Arc/User Data/Default/Cookies"), SafetyLevel::Caution),
            ("Arc History", home.join("Library/Application Support/Arc/User Data/Default/History"), SafetyLevel::Caution),
            
            ("Vivaldi Cookies", home.join("Library/Application Support/Vivaldi/Default/Cookies"), SafetyLevel::Caution),
            ("Vivaldi History", home.join("Library/Application Support/Vivaldi/Default/History"), SafetyLevel::Caution),
            
            ("Opera Cookies", home.join("Library/Application Support/com.operasoftware.Opera/Cookies"), SafetyLevel::Caution),
            ("Opera History", home.join("Library/Application Support/com.operasoftware.Opera/History"), SafetyLevel::Caution),
            
            ("Recent Items", home.join("Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments/com.apple.LSSharedFileList.ApplicationRecentDocuments.sfl"), SafetyLevel::Caution),
            ("Recent Servers", home.join("Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.RecentServers.sfl"), SafetyLevel::Caution),
            
            ("Download History", home.join("Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist"), SafetyLevel::Caution),
            
            ("Quick Look Cache", home.join("Library/Caches/com.apple.QuickLookDaemon/Cache.db"), SafetyLevel::Safe),
            ("Finder Recent", home.join("Library/Preferences/com.apple.finder.plist"), SafetyLevel::Caution),
        ];

        Self { search_paths }
    }

    fn get_file_size(path: &std::path::Path) -> u64 {
        if path.exists() {
            if path.is_file() {
                path.metadata().map(|m| m.len()).unwrap_or(0)
            } else if path.is_dir() {
                walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.metadata().ok())
                    .filter(|m| m.is_file())
                    .map(|m| m.len())
                    .sum()
            } else {
                0
            }
        } else {
            0
        }
    }
}

impl Scanner for PrivacyScanner {
    fn id(&self) -> &str {
        "privacy"
    }

    fn name(&self) -> &str {
        "Privacy"
    }

    fn category(&self) -> ScannerCategory {
        ScannerCategory::Browser
    }

    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut items = Vec::new();

        for (label, path, safety) in &self.search_paths {
            let actual_path = if path.display().to_string().contains("Firefox/Profiles/") {
                let profiles_dir = path.parent().unwrap().parent().unwrap();
                if !profiles_dir.exists() {
                    continue;
                }
                if let Some(profile) = std::fs::read_dir(profiles_dir)
                    .ok()
                    .and_then(|mut d| d.next())
                    .and_then(|e| e.ok())
                {
                    let filename = path.file_name().unwrap().to_str().unwrap();
                    profile.path().join(filename)
                } else {
                    continue;
                }
            } else {
                path.clone()
            };

            if !actual_path.exists() {
                continue;
            }

            config.report_progress(&actual_path.display().to_string());

            if config
                .excluded_paths
                .iter()
                .any(|ex| actual_path.starts_with(ex))
            {
                continue;
            }

            let size = Self::get_file_size(&actual_path);
            if size < config.min_size {
                continue;
            }

            let mut item = ScanResult::new(
                format!("privacy_{}", items.len()),
                label.to_string(),
                actual_path.clone(),
            )
            .with_size(size)
            .with_file_count(1)
            .with_category(ScannerCategory::Browser)
            .with_safety(*safety)
            .with_last_accessed(
                actual_path
                    .metadata()
                    .ok()
                    .and_then(|m| m.accessed().ok())
                    .map(|t| chrono::DateTime::from(t)),
            )
            .with_last_modified(
                actual_path
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::from(t)),
            );

            item.metadata
                .insert("scanner_id".to_string(), self.id().to_string());

            config.report_item(item.clone());
            items.push(item);
        }

        items.sort_by(|a, b| b.size.cmp(&a.size));
        Ok(items)
    }

    fn is_available(&self) -> bool {
        self.search_paths.iter().any(|(_, p, _)| p.exists())
    }
}

impl Default for PrivacyScanner {
    fn default() -> Self {
        Self::new()
    }
}
