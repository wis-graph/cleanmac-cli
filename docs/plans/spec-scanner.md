# 스캐너 상세 스펙

## 개요

각 카테고리별 스캐너의 상세 스펙 정의

## 1. 시스템 캐시 스캐너

### CacheScanner

| 항목 | 값 |
|-----|-----|
| ID | `system_caches` |
| 카테고리 | System |
| 아이콘 | `` |

### 스캔 대상

```
~/Library/Caches/
├── com.apple.*          # Apple 시스템 앱 캐시
├── com.google.*         # Google 앱 캐시
├── com.microsoft.*      # Microsoft 앱 캐시
├── com.adobe.*          # Adobe 앱 캐시
└── [기타 앱 캐시]
```

### 제외 대상

```
~/Library/Caches/com.apple.bird   # iCloud 관련
~/Library/Caches/com.apple.QuickLookThumbnailCache  # 썸네일
```

### 결과 예시

```json
{
  "id": "cache_safari_001",
  "name": "com.apple.Safari",
  "path": "/Users/wis/Library/Caches/com.apple.Safari",
  "size": 823456789,
  "file_count": 1234,
  "dir_count": 56,
  "safety_level": "Safe"
}
```

---

## 2. 로그 스캐너

### LogScanner

| 항목 | 값 |
|-----|-----|
| ID | `system_logs` |
| 카테고리 | System |
| 아이콘 | `` |

### 스캔 대상

```
~/Library/Logs/
├── DiagnosticReports/   # 크래시 리포트
├── com.apple.*          # Apple 앱 로그
├── [앱별 로그]
└── *.log                # 개별 로그 파일

/private/var/log/
├── system.log           # 시스템 로그 (권한 필요)
├── install.log          # 설치 로그
└── daily/               # 일일 로그 아카이브
```

### 안전 레벨

- `~/Library/Logs/*`: Safe
- `/private/var/log/*`: Caution (시스템 로그)

---

## 3. 브라우저 캐시 스캐너

### SafariCacheScanner

| 항목 | 값 |
|-----|-----|
| ID | `browser_safari` |
| 카테고리 | Browser |
| 아이콘 | `` |

### 스캔 대상

```
~/Library/Caches/com.apple.Safari/
├── Webkit/
│   └── Cache.db         # 메인 캐시
└── com.apple.Safari/

~/Library/Safari/
├── History.db           # 방문 기록 (옵션)
├── Downloads.plist      # 다운로드 기록 (옵션)
└── ReadingListArchives/ # 리딩리스트 (옵션)
```

### ChromeCacheScanner

| 항목 | 값 |
|-----|-----|
| ID | `browser_chrome` |
| 카테고리 | Browser |
| 아이콘 | `` |

### 스캔 대상

```
~/Library/Caches/Google/Chrome/
├── Default/
│   ├── Cache/
│   ├── Code Cache/
│   └── GPUCache/
└── [Profile]/

~/Library/Application Support/Google/Chrome/
├── Default/
│   ├── History          # 방문 기록 (옵션)
│   ├── Cookies          # 쿠키 (옵션)
│   └── Login Data       # 저장된 비밀번호 (옵션)
```

### FirefoxCacheScanner

| 항목 | 값 |
|-----|-----|
| ID | `browser_firefox` |
| 카테고리 | Browser |
| 아이콘 | `` |

### 스캔 대상

```
~/Library/Caches/Firefox/
└── Profiles/[profile]/
    └── cache2/

~/Library/Application Support/Firefox/
└── Profiles/[profile]/
    ├── places.sqlite    # 방문 기록 (옵션)
    └── cookies.sqlite   # 쿠키 (옵션)
```

---

## 4. 개발 정크 스캐너

### DevJunkScanner

| 항목 | 값 |
|-----|-----|
| ID | `dev_junk` |
| 카테고리 | Development |
| 아이콘 | `` |

### 스캔 패턴

```rust
const DEV_PATTERNS: &[(&str, &str)] = &[
    // JavaScript/TypeScript
    ("node_modules", "**/node_modules"),
    
    // Rust
    ("target", "**/target"),
    
    // Java/Kotlin
    (".gradle", "**/.gradle"),
    ("build", "**/build"),
    (".m2", "~/.m2/repository"),
    
    // Python
    ("__pycache__", "**/__pycache__"),
    (".venv", "**/.venv"),
    ("venv", "**/venv"),
    (".cache", "**/.cache"),
    
    // Go
    ("pkg", "~/go/pkg"),
    
    // Swift/iOS
    ("DerivedData", "~/Library/Developer/Xcode/DerivedData"),
    (".build", "**/.build"),
    
    // C/C++
    ("cmake-build-debug", "**/cmake-build-debug"),
    ("cmake-build-release", "**/cmake-build-release"),
];
```

### 스캔 범위

```
기본 검색 경로:
- ~/Documents/
- ~/Projects/
- ~/Developer/
- ~/Workspace/
- ~/src/
- ~/code/

사용자 설정으로 추가 가능
```

### 결과 정렬

크기 내림차순 정렬, 상위 50개만 표시

---

## 5. 휴지통 스캐너

### TrashScanner

| 항목 | 값 |
|-----|-----|
| ID | `trash` |
| 카테고리 | Trash |
| 아이콘 | `` |

### 스캔 대상

```
~/.Trash/               # 사용자 휴지통
/Volumes/*/.Trashes/    # 외장 드라이브 휴지통 (권한 필요)
```

### 특이사항

- 항상 Safe 레벨
- 삭제 시 즉시 비워짐 (복구 불가)
- 외장 드라이브는 Full Disk Access 필요

---

## 6. 앱 스캐너

### AppScanner

| 항목 | 값 |
|-----|-----|
| ID | `apps` |
| 카테고리 | Apps |
| 아이콘 | `` |

### 스캔 대상

```
/Applications/           # 시스템 앱
~/Applications/         # 사용자 앱
```

### 앱 정보 추출

```rust
pub struct AppInfo {
    pub name: String,
    pub bundle_id: String,
    pub version: String,
    pub size: u64,
    pub install_date: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub category: AppCategory,
}

pub enum AppCategory {
    System,      // Apple 시스템 앱
    Productivity,// 생산성
    Developer,   // 개발 도구
    Game,        // 게임
    Utility,     // 유틸리티
    Other,       // 기타
}
```

### 미사용 앱 감지

- 90일 이상 실행하지 않은 앱 표시
- `last_used`는 `lsappinfo` 또는 Spotlight 메타데이터 활용

---

## 병렬 스캔 구현

```rust
use rayon::prelude::*;

pub fn parallel_scan(scanners: &[Box<dyn Scanner>], config: &ScanConfig) -> Vec<ScanResult> {
    scanners
        .par_iter()
        .filter(|s| s.is_available())
        .flat_map(|scanner| {
            scanner.scan(config).unwrap_or_default()
        })
        .collect()
}

// 진행률 표시용
pub fn parallel_scan_with_progress(
    scanners: &[Box<dyn Scanner>],
    config: &ScanConfig,
    progress_tx: mpsc::Sender<ScanProgress>,
) -> Vec<ScanResult> {
    scanners
        .par_iter()
        .filter(|s| s.is_available())
        .flat_map(|scanner| {
            let _ = progress_tx.send(ScanProgress::Started(scanner.name().to_string()));
            let results = scanner.scan(config).unwrap_or_default();
            let _ = progress_tx.send(ScanProgress::Completed {
                scanner: scanner.name().to_string(),
                count: results.len(),
            });
            results
        })
        .collect()
}

pub enum ScanProgress {
    Started(String),
    Completed { scanner: String, count: usize },
}
```

---

**버전**: 1.0
**작성일**: 2026-02-18
