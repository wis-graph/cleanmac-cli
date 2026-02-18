# 플러그인 시스템 상세 스펙

## 개요

정적 트레이트 기반 플러그인 시스템. 컴파일 타임에 모든 플러그인이 등록되며, 런타임 동적 로딩은 없음.

## 핵심 트레이트

### Scanner 트레이트

```rust
pub trait Scanner: Send + Sync {
    /// 플러그인 고유 식별자
    fn id(&self) -> &str;
    
    /// 표시 이름
    fn name(&self) -> &str;
    
    /// 카테고리 (UI 그룹핑용)
    fn category(&self) -> ScannerCategory;
    
    /// 아이콘 (TUI 표시용)
    fn icon(&self) -> &str;
    
    /// 스캔 실행
    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>>;
    
    /// 스캔 가능 여부 (권한 등 확인)
    fn is_available(&self) -> bool;
    
    /// 예상 소요 시간 (UI 표시용)
    fn estimated_duration(&self) -> Duration;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScannerCategory {
    System,     // 시스템 캐시, 로그
    Browser,    // 브라우저 캐시
    Development,// 개발 정크
    Apps,       // 앱 관련
    Trash,      // 휴지통
}

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub min_size: u64,           // 최소 파일 크기
    pub max_depth: usize,        // 최대 디렉토리 깊이
    pub excluded_paths: Vec<PathBuf>,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub id: String,              // 고유 ID
    pub name: String,            // 표시 이름
    pub path: PathBuf,           // 전체 경로
    pub size: u64,               // 바이트 크기
    pub file_count: u64,         // 포함된 파일 수
    pub dir_count: u64,          // 포함된 디렉토리 수
    pub last_accessed: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub safety_level: SafetyLevel,
    pub metadata: HashMap<String, String>,  // 추가 메타데이터
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyLevel {
    Safe,        // 안전하게 삭제 가능
    Caution,     // 주의 필요
    Protected,   // 삭제 불가
}
```

### Cleaner 트레이트

```rust
pub trait Cleaner: Send + Sync {
    /// 삭제 실행
    fn clean(&self, items: &[ScanResult], config: &CleanConfig) -> Result<CleanResult>;
    
    /// 삭제 가능 여부 확인
    fn can_clean(&self, item: &ScanResult) -> bool;
}

#[derive(Debug, Clone)]
pub struct CleanConfig {
    pub dry_run: bool,           // 실제 삭제 여부
    pub create_backup: bool,     // 백업 생성 여부
    pub log_history: bool,       // 히스토리 기록 여부
}

#[derive(Debug)]
pub struct CleanResult {
    pub success_count: usize,
    pub failed_count: usize,
    pub total_freed: u64,
    pub failed_items: Vec<(PathBuf, String)>,
    pub duration: Duration,
}
```

## 플러그인 레지스트리

```rust
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
    
    /// 스캐너 등록
    pub fn register_scanner(&mut self, scanner: Box<dyn Scanner>) {
        self.scanners.push(scanner);
    }
    
    /// 카테고리별 스캐너 조회
    pub fn scanners_by_category(&self, category: ScannerCategory) -> Vec<&dyn Scanner> {
        self.scanners
            .iter()
            .filter(|s| s.category() == category)
            .map(|s| s.as_ref())
            .collect()
    }
    
    /// 전체 스캔 (병렬)
    pub fn scan_all(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        use rayon::prelude::*;
        
        self.scanners
            .par_iter()
            .filter(|s| s.is_available())
            .flat_map(|s| s.scan(config).unwrap_or_default())
            .collect()
    }
}

// 기본 플러그인 등록
impl Default for PluginRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        
        // 시스템
        registry.register_scanner(Box::new(CacheScanner::new()));
        registry.register_scanner(Box::new(LogScanner::new()));
        registry.register_scanner(Box::new(TrashScanner::new()));
        
        // 브라우저
        registry.register_scanner(Box::new(SafariCacheScanner::new()));
        registry.register_scanner(Box::new(ChromeCacheScanner::new()));
        registry.register_scanner(Box::new(FirefoxCacheScanner::new()));
        
        // 개발
        registry.register_scanner(Box::new(DevJunkScanner::new()));
        
        registry
    }
}
```

## 플러그인 구현 예시

```rust
pub struct CacheScanner {
    cache_dirs: Vec<PathBuf>,
}

impl CacheScanner {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap();
        Self {
            cache_dirs: vec![
                home.join("Library/Caches"),
            ],
        }
    }
}

impl Scanner for CacheScanner {
    fn id(&self) -> &str { "system_caches" }
    fn name(&self) -> &str { "System Caches" }
    fn category(&self) -> ScannerCategory { ScannerCategory::System }
    fn icon(&self) -> &str { "" }
    
    fn scan(&self, config: &ScanConfig) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();
        
        for cache_dir in &self.cache_dirs {
            if !cache_dir.exists() { continue; }
            
            for entry in WalkDir::new(cache_dir)
                .max_depth(config.max_depth)
                .into_iter()
                .filter_entry(|e| !is_excluded(e.path(), &config.excluded_paths))
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_dir() {
                    let size = calculate_dir_size(entry.path());
                    if size >= config.min_size {
                        results.push(ScanResult {
                            id: generate_id(),
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: entry.path().to_path_buf(),
                            size,
                            file_count: count_files(entry.path()),
                            dir_count: count_dirs(entry.path()),
                            last_accessed: get_last_accessed(entry.path()),
                            last_modified: get_last_modified(entry.path()),
                            safety_level: SafetyLevel::Safe,
                            metadata: HashMap::new(),
                        });
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    fn is_available(&self) -> bool { true }
    fn estimated_duration(&self) -> Duration { Duration::from_secs(5) }
}
```

## 확장 가이드

새로운 스캐너 추가:

1. `Scanner` 트레이트 구현
2. `PluginRegistry::default()`에 등록
3. 필요시 `ScannerCategory` 확장

```rust
// 1. 새 스캐너 구현
pub struct MyCustomScanner;

impl Scanner for MyCustomScanner {
    // ... 트레이트 구현
}

// 2. 레지스트리에 등록
impl Default for PluginRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        // ... 기존 스캐너들
        registry.register_scanner(Box::new(MyCustomScanner));
        registry
    }
}
```

---

**버전**: 1.0
**작성일**: 2026-02-18
