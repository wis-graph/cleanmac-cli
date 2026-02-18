# 앱 삭제 상세 스펙

## 개요

CleanMyMac 수준의 완전한 앱 삭제 기능 구현

## 워크플로우

```
1. 앱 검색 (이름 또는 경로)
2. 번들 정보 추출 (Info.plist)
3. 관련 파일 검색
4. 사용자 확인
5. 삭제 실행
6. 히스토리 기록
```

## 1. 앱 검색

### AppDetector

```rust
pub struct AppDetector {
    search_paths: Vec<PathBuf>,
}

impl AppDetector {
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("/Applications"),
                dirs::home_dir().unwrap().join("Applications"),
            ],
        }
    }
    
    /// 이름으로 앱 검색
    pub fn find_by_name(&self, name: &str) -> Option<AppBundle> {
        for path in &self.search_paths {
            for entry in fs::read_dir(path).ok()? {
                let entry = entry.ok()?;
                let app_name = entry.file_name().to_string_lossy().to_string();
                
                // 부분 일치 허용
                if app_name.to_lowercase().contains(&name.to_lowercase()) {
                    return Some(AppBundle::new(entry.path()));
                }
            }
        }
        None
    }
    
    /// 전체 앱 목록
    pub fn list_all(&self) -> Vec<AppBundle> {
        let mut apps = Vec::new();
        
        for path in &self.search_paths {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "app").unwrap_or(false) {
                        apps.push(AppBundle::new(path));
                    }
                }
            }
        }
        
        apps
    }
}
```

## 2. 번들 정보 추출

### AppBundle

```rust
pub struct AppBundle {
    pub path: PathBuf,
    pub info: Option<PlistInfo>,
}

#[derive(Debug, Clone)]
pub struct PlistInfo {
    pub bundle_id: String,           // CFBundleIdentifier
    pub name: String,                // CFBundleName
    pub display_name: String,        // CFBundleDisplayName
    pub version: String,             // CFBundleShortVersionString
    pub build_version: String,       // CFBundleVersion
    pub minimum_os: String,          // LSMinimumSystemVersion
    pub signature: String,           // CFBundleSignature
    pub icon_file: String,           // CFBundleIconFile
}

impl AppBundle {
    pub fn new(path: PathBuf) -> Self {
        let info = Self::parse_plist(&path).ok();
        Self { path, info }
    }
    
    fn parse_plist(path: &Path) -> Result<PlistInfo> {
        let plist_path = path.join("Contents/Info.plist");
        let content = fs::read(&plist_path)?;
        
        // plist 파싱 (plist crate 사용)
        let plist = plist::Value::from_reader(Cursor::new(content))?;
        
        Ok(PlistInfo {
            bundle_id: plist.as_dictionary()
                .and_then(|d| d.get("CFBundleIdentifier"))
                .and_then(|v| v.as_string())
                .unwrap_or("")
                .to_string(),
            // ... 기타 필드
        })
    }
    
    pub fn size(&self) -> u64 {
        calculate_dir_size(&self.path)
    }
}
```

## 3. 관련 파일 검색

### RelatedFileDetector

```rust
pub struct RelatedFileDetector {
    home: PathBuf,
}

impl RelatedFileDetector {
    pub fn new() -> Self {
        Self {
            home: dirs::home_dir().unwrap(),
        }
    }
    
    /// 앱과 관련된 모든 파일 검색
    pub fn find_related_files(&self, app: &AppBundle) -> Vec<RelatedFile> {
        let mut files = Vec::new();
        
        let app_name = app.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        let bundle_id = app.info.as_ref()
            .map(|i| i.bundle_id.as_str())
            .unwrap_or("");
        
        // 검색할 위치들
        let search_locations = self.get_search_locations();
        
        for (category, location) in search_locations {
            if let Ok(entries) = fs::read_dir(&location) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    
                    if self.is_related(&name, app_name, bundle_id) {
                        files.push(RelatedFile {
                            path: entry.path(),
                            category: category.clone(),
                            size: self.calculate_size(&entry.path()),
                        });
                    }
                }
            }
        }
        
        files
    }
    
    fn get_search_locations(&self) -> Vec<(RelatedCategory, PathBuf)> {
        vec![
            // 사용자 라이브러리
            (RelatedCategory::AppSupport, self.home.join("Library/Application Support")),
            (RelatedCategory::Preferences, self.home.join("Library/Preferences")),
            (RelatedCategory::Caches, self.home.join("Library/Caches")),
            (RelatedCategory::Logs, self.home.join("Library/Logs")),
            (RelatedCategory::LaunchAgents, self.home.join("Library/LaunchAgents")),
            (RelatedCategory::Containers, self.home.join("Library/Containers")),
            (RelatedCategory::GroupContainers, self.home.join("Library/Group Containers")),
            (RelatedCategory::Cookies, self.home.join("Library/Cookies")),
            (RelatedCategory::WebKit, self.home.join("Library/WebKit")),
            
            // 시스템 라이브러리 (권한 필요)
            (RelatedCategory::LaunchDaemons, PathBuf::from("/Library/LaunchDaemons")),
            (RelatedCategory::SystemAppSupport, PathBuf::from("/Library/Application Support")),
            
            // 기타
            (RelatedCategory::Fonts, self.home.join("Library/Fonts")),
        ]
    }
    
    fn is_related(&self, name: &str, app_name: &str, bundle_id: &str) -> bool {
        let name_lower = name.to_lowercase();
        let app_lower = app_name.to_lowercase();
        let bundle_lower = bundle_id.to_lowercase();
        
        // 번들 ID로 매칭 (가장 정확)
        if !bundle_id.is_empty() && name_lower.contains(&bundle_lower) {
            return true;
        }
        
        // 앱 이름으로 매칭
        if !app_name.is_empty() && name_lower.contains(&app_lower) {
            return true;
        }
        
        // Preferences plist 파일 (com.company.app.plist 형식)
        if name.ends_with(".plist") && !bundle_id.is_empty() {
            if name_lower.starts_with(&bundle_lower.replace(".", "")) {
                return true;
            }
        }
        
        false
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct RelatedFile {
    pub path: PathBuf,
    pub category: RelatedCategory,
    pub size: u64,
}
```

## 4. 삭제 실행

### Uninstaller

```rust
pub struct Uninstaller {
    dry_run: bool,
    safety_checker: SafetyChecker,
    history_logger: HistoryLogger,
}

impl Uninstaller {
    pub fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            safety_checker: SafetyChecker::new(),
            history_logger: HistoryLogger::new(),
        }
    }
    
    /// 앱 완전 삭제
    pub fn uninstall(&self, app: &AppBundle, related: &[RelatedFile]) -> Result<UninstallResult> {
        let mut result = UninstallResult::new();
        
        // 1. 실행 중인지 확인
        if self.is_running(app)? {
            return Err(anyhow!("App is currently running"));
        }
        
        // 2. 시스템 앱인지 확인
        if self.is_system_app(app) {
            return Err(anyhow!("Cannot uninstall system app"));
        }
        
        // 3. 앱 번들 삭제
        match self.delete_path(&app.path) {
            Ok(()) => {
                result.deleted_app = true;
                result.total_freed += app.size();
                self.log_deletion(&app.path)?;
            }
            Err(e) => result.errors.push(format!("Failed to delete app: {}", e)),
        }
        
        // 4. 관련 파일 삭제
        for file in related {
            if self.safety_checker.is_safe_to_delete(&file.path) {
                match self.delete_path(&file.path) {
                    Ok(()) => {
                        result.deleted_related.push(file.path.clone());
                        result.total_freed += file.size;
                        self.log_deletion(&file.path)?;
                    }
                    Err(e) => result.errors.push(format!("Failed to delete {}: {}", 
                        file.path.display(), e)),
                }
            } else {
                result.skipped.push(file.path.clone());
            }
        }
        
        result.dry_run = self.dry_run;
        Ok(result)
    }
    
    fn delete_path(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            println!("[DRY-RUN] Would delete: {}", path.display());
            return Ok(());
        }
        
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }
    
    fn is_running(&self, app: &AppBundle) -> Result<bool> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get name of every process")
            .output()?;
        
        let running = String::from_utf8_lossy(&output.stdout);
        let app_name = app.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        Ok(running.to_lowercase().contains(&app_name.to_lowercase()))
    }
    
    fn is_system_app(&self, app: &AppBundle) -> bool {
        let system_apps = [
            "com.apple.Safari",
            "com.apple.Mail",
            "com.apple.calendar",
            "com.apple.AddressBook",
            "com.apple.finder",
            // ... 기타 시스템 앱
        ];
        
        app.info.as_ref()
            .map(|i| system_apps.contains(&i.bundle_id.as_str()))
            .unwrap_or(false)
    }
    
    fn log_deletion(&self, path: &Path) -> Result<()> {
        if !self.dry_run {
            self.history_logger.log(LogEntry {
                action: "DELETE".to_string(),
                path: path.to_path_buf(),
                timestamp: Utc::now(),
            })?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct UninstallResult {
    pub dry_run: bool,
    pub deleted_app: bool,
    pub deleted_related: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<String>,
    pub total_freed: u64,
}
```

## 5. TUI 화면 예시

```
┌─────────────────────────────────────────────────────────────────┐
│  Uninstall: Visual Studio Code                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Application                                                     │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ [x] Visual Studio Code.app     450 MB                       ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  Related Files                                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ [x] ~/Library/Application Support/Code     120 MB           ││
│  │ [x] ~/Library/Caches/com.microsoft.VSCode   80 MB           ││
│  │ [x] ~/Library/Preferences/com.microsoft...   2 KB           ││
│  │ [x] ~/Library/HTTPStorages/com.microsoft...   5 MB          ││
│  │ [ ] ~/Library/Containers/com.microsoft...  (Protected)      ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  Total to free: 655 MB                                           │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  [Enter] Execute  [Space] Toggle  [Esc] Cancel                   │
└─────────────────────────────────────────────────────────────────┘
```

---

**버전**: 1.0
**작성일**: 2026-02-18
