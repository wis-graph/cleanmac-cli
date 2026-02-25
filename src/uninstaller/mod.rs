use anyhow::Result;
use plist::Value;
use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct PlistInfo {
    pub bundle_id: String,
    pub version: String,
}

pub struct AppBundle {
    pub path: PathBuf,
    info: RefCell<Option<PlistInfo>>,
    cached_size: Cell<Option<u64>>,
}

impl Clone for AppBundle {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            info: RefCell::new(self.info.borrow().clone()),
            cached_size: Cell::new(self.cached_size.get()),
        }
    }
}

impl std::fmt::Debug for AppBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppBundle")
            .field("path", &self.path)
            .field("info", &self.info.borrow())
            .finish()
    }
}

impl AppBundle {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            info: RefCell::new(None),
            cached_size: Cell::new(None),
        }
    }

    pub fn info(&self) -> Option<PlistInfo> {
        if self.info.borrow().is_none() {
            if let Ok(parsed) = Self::parse_plist(&self.path) {
                *self.info.borrow_mut() = Some(parsed);
            }
        }
        self.info.borrow().clone()
    }

    fn parse_plist(path: &Path) -> Result<PlistInfo> {
        let plist_path = path.join("Contents/Info.plist");
        let content = fs::read(&plist_path)?;
        let plist = Value::from_reader(Cursor::new(content))?;

        let get_string = |key: &str| -> String {
            plist
                .as_dictionary()
                .and_then(|d| d.get(key))
                .and_then(|v| v.as_string())
                .unwrap_or("")
                .to_string()
        };

        Ok(PlistInfo {
            bundle_id: get_string("CFBundleIdentifier"),
            version: get_string("CFBundleShortVersionString"),
        })
    }

    pub fn size(&self) -> u64 {
        if let Some(size) = self.cached_size.get() {
            return size;
        }
        let size = calculate_dir_size(&self.path);
        self.cached_size.set(Some(size));
        size
    }

    pub fn name(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
    }
}

fn calculate_dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }

    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelatedCategory {
    AppSupport,
    Preferences,
    Caches,
    Logs,
    LaunchAgents,
    LaunchDaemons,
    Containers,
    GroupContainers,
    Cookies,
    WebKit,
    Fonts,
    SystemAppSupport,
}

impl RelatedCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            RelatedCategory::AppSupport => "Application Support",
            RelatedCategory::Preferences => "Preferences",
            RelatedCategory::Caches => "Caches",
            RelatedCategory::Logs => "Logs",
            RelatedCategory::LaunchAgents => "Launch Agents",
            RelatedCategory::LaunchDaemons => "Launch Daemons",
            RelatedCategory::Containers => "Containers",
            RelatedCategory::GroupContainers => "Group Containers",
            RelatedCategory::Cookies => "Cookies",
            RelatedCategory::WebKit => "WebKit",
            RelatedCategory::Fonts => "Fonts",
            RelatedCategory::SystemAppSupport => "System Application Support",
        }
    }

    pub fn is_protected(&self) -> bool {
        matches!(
            self,
            RelatedCategory::LaunchDaemons
                | RelatedCategory::SystemAppSupport
                | RelatedCategory::Containers
        )
    }
}

#[derive(Debug, Clone)]
pub struct RelatedFile {
    pub path: PathBuf,
    pub category: RelatedCategory,
    pub size: u64,
}

pub struct AppDetector {
    search_paths: Vec<PathBuf>,
}

impl AppDetector {
    pub fn new() -> Self {
        let mut search_paths = vec![PathBuf::from("/Applications")];

        if let Some(home) = dirs::home_dir() {
            search_paths.push(home.join("Applications"));
        }

        Self { search_paths }
    }

    pub fn find_by_name(&self, name: &str) -> Option<AppBundle> {
        let name_lower = name.to_lowercase();

        for path in &self.search_paths {
            if !path.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let app_name = entry.file_name().to_string_lossy().to_string();
                    if app_name.to_lowercase().contains(&name_lower) {
                        return Some(AppBundle::new(entry.path()));
                    }
                }
            }
        }

        None
    }

    pub fn list_all(&self) -> Vec<AppBundle> {
        let mut apps = Vec::new();

        for path in &self.search_paths {
            if !path.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.extension().map(|e| e == "app").unwrap_or(false) {
                        apps.push(AppBundle::new(entry_path));
                    }
                }
            }
        }

        apps.sort_by(|a, b| a.name().to_lowercase().cmp(&b.name().to_lowercase()));
        apps
    }
}

impl Default for AppDetector {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RelatedFileDetector {
    home: PathBuf,
}

impl RelatedFileDetector {
    pub fn new() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }

    pub fn find_related_files(&self, app: &AppBundle) -> Vec<RelatedFile> {
        let mut files = Vec::new();

        let app_name = app.name();
        let bundle_id = app.info().map(|i| i.bundle_id.clone()).unwrap_or_default();

        let search_locations = self.get_search_locations();

        for (category, location) in search_locations {
            if !location.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&location) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let path = entry.path();

                    if self.is_related(&name, app_name, &bundle_id) {
                        files.push(RelatedFile {
                            path: path.clone(),
                            category,
                            size: calculate_dir_size(&path),
                        });
                    }
                }
            }
        }

        files
    }

    fn get_search_locations(&self) -> Vec<(RelatedCategory, PathBuf)> {
        vec![
            (
                RelatedCategory::AppSupport,
                self.home.join("Library/Application Support"),
            ),
            (
                RelatedCategory::Preferences,
                self.home.join("Library/Preferences"),
            ),
            (RelatedCategory::Caches, self.home.join("Library/Caches")),
            (RelatedCategory::Logs, self.home.join("Library/Logs")),
            (
                RelatedCategory::LaunchAgents,
                self.home.join("Library/LaunchAgents"),
            ),
            (
                RelatedCategory::Containers,
                self.home.join("Library/Containers"),
            ),
            (
                RelatedCategory::GroupContainers,
                self.home.join("Library/Group Containers"),
            ),
            (RelatedCategory::Cookies, self.home.join("Library/Cookies")),
            (RelatedCategory::WebKit, self.home.join("Library/WebKit")),
            (RelatedCategory::Fonts, self.home.join("Library/Fonts")),
            (
                RelatedCategory::LaunchDaemons,
                PathBuf::from("/Library/LaunchDaemons"),
            ),
            (
                RelatedCategory::SystemAppSupport,
                PathBuf::from("/Library/Application Support"),
            ),
        ]
    }

    fn is_related(&self, name: &str, app_name: &str, bundle_id: &str) -> bool {
        let name_lower = name.to_lowercase();
        let app_lower = app_name.to_lowercase();
        let bundle_lower = bundle_id.to_lowercase();

        if !bundle_id.is_empty() && name_lower.contains(&bundle_lower) {
            return true;
        }

        if !app_name.is_empty() && name_lower.contains(&app_lower) {
            return true;
        }

        if name.ends_with(".plist") && !bundle_id.is_empty() {
            let bundle_prefix = bundle_lower.replace(".", "");
            if name_lower.starts_with(&bundle_prefix) {
                return true;
            }
        }

        false
    }
}

impl Default for RelatedFileDetector {
    fn default() -> Self {
        Self::new()
    }
}

const SYSTEM_APPS: &[&str] = &[
    "com.apple.Safari",
    "com.apple.Mail",
    "com.apple.calendar",
    "com.apple.AddressBook",
    "com.apple.finder",
    "com.apple.Terminal",
    "com.apple.Preview",
    "com.apple.TextEdit",
    "com.apple.Notes",
    "com.apple.Reminders",
    "com.apple.Maps",
    "com.apple.Photos",
    "com.apple.Music",
    "com.apple.Podcasts",
    "com.apple.News",
    "com.apple.Stocks",
    "com.apple.FaceTime",
    "com.apple.Messages",
    "com.apple.AppStore",
    "com.apple.SystemPreferences",
    "com.apple.Utilities",
];

pub struct Uninstaller {
    dry_run: bool,
}

impl Uninstaller {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    pub fn is_system_app(&self, app: &AppBundle) -> bool {
        app.info()
            .map(|i| SYSTEM_APPS.contains(&i.bundle_id.as_str()))
            .unwrap_or(false)
    }

    pub fn is_running(&self, app: &AppBundle) -> Result<bool> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get name of every process")
            .output()?;

        let running = String::from_utf8_lossy(&output.stdout).to_lowercase();
        let app_name = app.name().to_lowercase();

        Ok(running.contains(&app_name))
    }

    pub fn uninstall(
        &self,
        app: &AppBundle,
        related_files: &[RelatedFile],
    ) -> Result<UninstallResult> {
        let mut result = UninstallResult::new();

        if self.is_system_app(app) {
            result
                .errors
                .push("Cannot uninstall system app".to_string());
            return Ok(result);
        }

        if self.is_running(app)? {
            result
                .errors
                .push("App is currently running. Please quit the app first.".to_string());
            return Ok(result);
        }

        let app_size = app.size();
        if self.delete_path(&app.path)? {
            result.deleted_app = true;
            result.total_freed += app_size;
        } else {
            result
                .errors
                .push(format!("Failed to delete app: {}", app.path.display()));
        }

        for file in related_files {
            if file.category.is_protected() {
                result.skipped.push(file.path.clone());
                continue;
            }

            if self.delete_path(&file.path)? {
                result.deleted_related.push(file.path.clone());
                result.total_freed += file.size;
            } else {
                result
                    .errors
                    .push(format!("Failed to delete: {}", file.path.display()));
            }
        }

        result.dry_run = self.dry_run;
        Ok(result)
    }

    fn delete_path(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }

        if self.dry_run {
            println!("[DRY-RUN] Would delete: {}", path.display());
            return Ok(true);
        }

        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };

        match result {
            Ok(()) => {
                println!("Deleted: {}", path.display());
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                self.delete_with_admin_privileges(path)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn delete_with_admin_privileges(&self, path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();
        let script = if path.is_dir() {
            format!(
                "do shell script \"rm -rf '{}'\" with administrator privileges",
                path_str
            )
        } else {
            format!(
                "do shell script \"rm '{}'\" with administrator privileges",
                path_str
            )
        };

        let output = Command::new("osascript").arg("-e").arg(&script).output();

        match output {
            Ok(o) if o.status.success() => {
                println!("Deleted (with admin): {}", path.display());
                Ok(true)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                anyhow::bail!("Admin privileges denied or failed: {}", stderr);
            }
            Err(e) => anyhow::bail!("Failed to request admin privileges: {}", e),
        }
    }
}

#[derive(Debug, Default)]
pub struct UninstallResult {
    pub dry_run: bool,
    pub deleted_app: bool,
    pub deleted_related: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<String>,
    pub total_freed: u64,
}

impl UninstallResult {
    fn new() -> Self {
        Self::default()
    }
}
