# CleanX CLI 설계 문서

## 개요

CleanMyMac의 핵심 기능을 CLI + TUI로 제공하는 안전하고 빠른 macOS 시스템 정리 도구

## 타겟 사용자

일반 사용자 - 복잡한 명령어 없이 시스템 정리를 원하는 macOS 사용자

## 최종 설계 결정사항

| 항목 | 결정사항 |
|------|----------|
| 타겟 | 일반 사용자용 전체 시스템 정리 |
| 언어 | Rust |
| 구조 | 플러그인 (정적 트레이트 기반) |
| UI | 대화형 TUI + 트리 뷰 + 상세 패널 |
| 앱 삭제 | 심화형 (전체 연관 파일) |
| 안전장치 | dry-run 우선, `--execute` 플래그 |
| 설정 | TOML (`~/.config/cleanx/config.toml`) |
| 히스토리 | 세션 로그 (`~/.local/share/cleanx/history.log`) |
| 스캔 | 병렬 처리 (rayon) |
| 선택 | 다중 선택 (Space 토글) |
| MVP 카테고리 | 캐시, 로그, 휴지통, 브라우저, 개발정크, 앱삭제 |


## 기술 스택

| 구성요소 | 기술 | 용도 |
|---------|------|------|
| 언어 | Rust | 안전성, 성능 |
| CLI 파싱 | `clap` | 명령행 인자 처리 |
| TUI | `ratatui` | 대화형 터미널 UI |
| 파일 순회 | `walkdir`, `glob` | 파일 시스템 스캔 |
| 병렬 처리 | `rayon` | 멀티스레드 스캔 |
| 설정 관리 | `serde` + `toml` | 설정 파일 |
| 에러 처리 | `anyhow`, `thiserror` | 에러 핸들링 |
| 크기 포맷 | `byte-unit` | 바이트 단위 변환 |


## 프로젝트 구조

```
cleanx/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs                    # clap 명령어 정의
│   ├── lib.rs                    # 라이브러리 진입점
│   ├── config.rs                 # 설정 관리
│   ├── safety.rs                 # 안전 검증
│   ├── history.rs                # 삭제 히스토리 로그
│   ├── plugin/                   # 플러그인 시스템
│   │   ├── mod.rs
│   │   └── trait.rs              # Scanner, Cleaner 트레이트 정의
│   ├── scanner/                  # 스캐너 플러그인 구현체
│   │   ├── mod.rs
│   │   ├── caches.rs             # 시스템 캐시
│   │   ├── logs.rs               # 로그 파일
│   │   ├── browser.rs            # 브라우저 캐시
│   │   ├── dev.rs                # 개발 정크
│   │   ├── trash.rs              # 휴지통
│   │   └── apps.rs               # 앱 검색
│   ├── cleaner/                  # 삭제 로직
│   │   ├── mod.rs
│   │   └── executor.rs           # 실제 삭제 실행
│   ├── uninstaller/              # 앱 완전 삭제
│   │   ├── mod.rs
│   │   ├── detector.rs           # 앱 검색
│   │   └── related_files.rs      # 연관 파일 검색
│   └── tui/                      # TUI UI
│       ├── mod.rs
│       ├── app.rs                # 메인 앱 상태
│       ├── components/
│       │   ├── mod.rs
│       │   ├── category_list.rs  # 카테고리 목록
│       │   ├── item_list.rs      # 아이템 목록
│       │   └── detail_panel.rs   # 상세 패널
│       └── event.rs              # 키 이벤트 처리
├── docs/
│   └── plans/
│       ├── 2026-02-18-cleanx-design.md
│       ├── spec-plugin.md        # 플러그인 시스템 상세 스펙
│       ├── spec-scanner.md       # 스캐너 상세 스펙
│       ├── spec-uninstaller.md   # 앱 삭제 상세 스펙
│       └── spec-tui.md           # TUI 상세 스펙
└── config/
    └── default.toml              # 기본 설정 예시
```

## 파일 저장 위치

```
~/.config/cleanx/
├── config.toml                   # 사용자 설정
└── plugins/                      # (향후) 외부 플러그인

~/.local/share/cleanx/
├── history.log                   # 삭제 히스토리
└── cache/                        # 스캔 캐시 (선택적)
```

## 핵심 기능

### 1. 스캔 카테고리 (MVP)

| 카테고리 | 스캔 대상 | 예상 크기 |
|---------|----------|----------|
| 시스템 캐시 | `~/Library/Caches` (시스템 관련) | 1-3 GB |
| 로그 | `~/Library/Logs`, `/var/log` | 100-500 MB |
| 휴지통 | `~/.Trash` | 가변 |
| 브라우저 캐시 | Safari, Chrome, Firefox 캐시 | 1-5 GB |
| 개발 정크 | `node_modules`, `target`, `.gradle` 등 | 5-20 GB |
| 앱 삭제 | `/Applications` + 관련 파일 전체 | 가변 |

### 2. 앱 완전 삭제 (심화형)

앱 삭제 시 다음 위치까지 검색하여 삭제:

```
~/Library/Application Support/[AppName]*
~/Library/Preferences/[AppName]*
~/Library/Caches/[AppName]*
~/Library/Logs/[AppName]*
~/Library/LaunchAgents/[AppName]*
/Library/LaunchDaemons/[AppBundleID]*
~/Library/Application Support/[AppBundleID]*
~/Library/Containers/[AppBundleID]*
~/Library/Group Containers/[AppBundleID]*
```

### 3. 안전 장치

- **기본 동작**: dry-run 모드로 미리보기
- **실제 삭제**: `--execute` 플래그 필요
- **보호 경로**: 시스템 필수 파일 삭제 방지
- **실행 중 앱**: 실행 중인 앱 삭제 방지

```
보호 경로:
- /System
- /usr (SIP 보호)
- /bin, /sbin, /etc
- ~/Library/Keychains
- ~/Library/Security
- 현재 실행 중인 앱
```

### 4. 삭제 히스토리

```
# ~/.local/share/cleanx/history.log 형식
2026-02-18T14:30:00Z DELETE /Users/wis/Library/Caches/com.apple.Safari
2026-02-18T14:30:01Z DELETE /Users/wis/Library/Logs/DiagnosticReports
2026-02-18T14:35:00Z DELETE /Applications/OldApp.app
```

## UI 설계

### TUI 레이아웃

```
┌──────────────────────────────────────────────────────────────┐
│  CleanX - 시스템 정리                          스캔: 5.2 GB  │
├────────────────────────────────┬─────────────────────────────┤
│  Categories                    │  Details                     │
│  ┌────────────────────────────┐│                              │
│  │ > System Caches (1.2 GB)   ││  Path:                       │
│  │   [x] Safari (800 MB)      ││  ~/Library/Caches/Safari     │
│  │   [ ] Chrome (400 MB)      ││                              │
│  │ Logs (200 MB)              ││  Size: 800 MB                │
│  │   [x] System Logs          ││  Files: 1,234                │
│  │ Dev Junk (3.8 GB)          ││  Last Access: 2026-02-15     │
│  │   [ ] node_modules         ││                              │
│  │ Trash (0 MB)               ││  [ ] Selected for deletion   │
│  └────────────────────────────┘│                              │
├────────────────────────────────┴─────────────────────────────┤
│  ↑↓: Navigate  Space: Select  Enter: Clean  Tab: Panel  q: Quit │
│  Selected: 2 items (1.0 GB)                                   │
└──────────────────────────────────────────────────────────────┘
```

### CLI 명령어

```bash
# TUI 실행 (기본)
cleanx

# 직접 스캔
cleanx scan --category caches
cleanx scan --category all

# 직접 정리 (dry-run)
cleanx clean --category caches

# 실제 정리
cleanx clean --category caches --execute

# 앱 삭제 (dry-run)
cleanx uninstall "App Name"

# 앱 삭제 (실제)
cleanx uninstall "App Name" --execute

# 설정
cleanx config show
cleanx config set min_size 1048576
cleanx config add-exclude "/Users/*/Important"

# 히스토리
cleanx history show
cleanx history clear
```

## 개발 단계

### Phase 1: 기본 구조 ✅
- [x] 프로젝트 초기화 (Cargo.toml)
- [x] 기본 CLI 구조 (clap)
- [x] 설정 파일 파싱 (serde + toml)
- [x] 히스토리 로깅

### Phase 2: 플러그인 시스템 ✅
- [x] Scanner 트레이트 정의
- [x] Cleaner 트레이트 정의
- [x] 플러그인 레지스트리

### Phase 3: 스캐너 구현 ✅
- [x] 시스템 캐시 스캐너
- [x] 로그 스캐너
- [x] 브라우저 캐시 스캐너 (13개 브라우저 지원)
- [x] 개발 정크 스캐너
- [x] 휴지통 스캐너
- [x] 병렬 스캔 구현

### Phase 4: TUI 구현 ✅
- [x] 기본 TUI 레이아웃 (ratatui)
- [x] 카테고리 목록 컴포넌트
- [x] 아이템 목록 컴포넌트
- [x] 상세 패널 컴포넌트
- [x] 다중 선택 로직 (a/n/Space)
- [x] 키보드 인터랙션
- [x] 진행률 표시
- [x] 포커스 시각화

### Phase 5: 삭제 기능 ✅
- [x] dry-run 모드
- [x] 실제 삭제 (--execute)
- [x] 안전 검증 로직
- [x] 히스토리 기록

### Phase 6: 앱 삭제 ✅
- [x] 앱 검색 및 메타데이터 추출 (plist 파싱)
- [x] 관련 파일 검색 (12개 카테고리)
- [x] 완전 삭제 구현
- [x] 시스템 앱 보호
- [x] 실행 중인 앱 감지
- [x] 백그라운드 용량 계산

## 제약사항

### 시스템 제약
- **SIP (System Integrity Protection)**: `/System`, `/usr` 등 직접 수정 불가
- **Full Disk Access**: 일부 영역 접근 위해 권한 안내 필요

### 권한 요구사항
- Full Disk Access (선택적, 일부 카테고리용)
- 사용자 홈 디렉토리 접근

## 참고 자료

- [ratatui](https://github.com/ratatui-org/ratatui) - Rust TUI 프레임워크
- [clap](https://github.com/clap-rs/clap) - CLI 파싱
- [walkdir](https://github.com/BurntSushi/walkdir) - 파일 시스템 순회
- [rayon](https://github.com/rayon-rs/rayon) - 데이터 병렬 처리
- CleanMyMac 기능 참고

---

**버전**: 2.0
**작성일**: 2026-02-18
**최종 업데이트**: 2026-02-18
**상태**: Phase 1-6 완료, Phase 7+ 계획 중 (roadmap.md 참조)
