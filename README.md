# CleanMac CLI

macOS 시스템 클리너 - CleanMyMac에서 영감을 받은 CLI + TUI 도구

**AI-Readable**: JSON 출력과 MCP 서버 지원으로 AI와 연동 가능

## Demo

![Demo](demo.gif)

## 설치

```bash
brew tap wis-graph/cleanmac
brew install cleanmac
```

또는 [GitHub Releases](https://github.com/wis-graph/cleanmac-cli/releases)에서 바이너리 다운로드

## 기능

### 시스템 정크
| 기능 | 설명 |
|------|------|
| 시스템 캐시 | `~/Library/Caches`, Xcode DerivedData |
| 시스템 로그 | `~/Library/Logs` |
| 휴지통 | `~/.Trash` |

### 브라우저
| 기능 | 설명 |
|------|------|
| 브라우저 캐시 | Safari, Chrome, Firefox, Edge, Arc, Brave, Vivaldi, Opera, Orion 등 12개 브라우저 지원 |
| 개인정보 | 쿠키, 방문 기록, 다운로드 기록 |

### 개발
| 기능 | 설명 |
|------|------|
| node_modules | 프로젝트별 node_modules |
| 빌드 아티팩트 | target/, build/, dist/, .next/ 등 |
| 패키지 매니저 캐시 | npm, yarn, pnpm, cargo, go |

### 미디어
| 기능 | 설명 |
|------|------|
| 사진 정크 | Photos Library 캐시, 미리보기 |
| 음악 정크 | iTunes/Music 캐시, 팟캐스트 |
| 메일 첨부파일 | Mail 다운로드 캐시 |

### 파일 관리
| 기능 | 설명 |
|------|------|
| 대용량/오래된 파일 | 30일+ 미사용 파일 검색 |
| 중복 파일 | 동일 내용 파일 검색 |

### 유지보수
| 기능 | 설명 |
|------|------|
| 시스템 유지보수 | DNS 캐시, Spotlight 인덱스, DYLD 캐시 등 |

### 시작 프로그램 관리
| 기능 | 설명 |
|------|------|
| LaunchAgents 스캔 | 사용자/시스템 LaunchAgents 조회 |
| LaunchDaemons 스캔 | 시스템 LaunchDaemons 조회 |
| Login Items 스캔 | 로그인 항목 조회 |
| 상세 정보 | 실행 파일 경로, 자동 실행 여부 표시 |

### 앱 삭제
| 기능 | 설명 |
|------|------|
| 완전 삭제 | 앱 + 관련 파일 (Preferences, Caches, Logs, Containers 등) |
| 관련 파일 검색 | 12개 카테고리 관련 파일 스캔 |

### 디스크 분석
| 기능 | 설명 |
|------|------|
| Space Lens | ncdu 스타일 디스크 사용량 시각화 |
| 실시간 스캔 | 폴더 크기 실시간 표시 |
| 병렬 처리 | 4/8/16 스레드 풀 지원 |

## 설치

### Homebrew (권장)

```bash
brew tap wis-graph/cleanmac
brew install cleanmac
```

### 수동 설치 (개발자)

```bash
git clone https://github.com/wis-graph/cleanmac-cli.git
cd cleanmac-cli
cargo build --release
sudo cp target/release/cleanmac /usr/local/bin/
```

## 사용법

### TUI 모드 (기본)
```bash
cleanmac              # TUI 진입
```

### CLI 명령어 (사람용)
```bash
# 스캔
cleanmac scan                     # 전체 스캔 (사람용)
cleanmac scan -c caches           # 특정 카테고리만

# 정리
cleanmac apply --category caches  # 드라이 런
cleanmac apply --category caches --yes  # 실제 삭제

# 디스크 분석
cleanmac space                    # 홈 디렉토리부터
cleanmac space -p /path           # 특정 경로
cleanmac space -t 8               # 8 스레드 사용

# 앱 삭제
cleanmac apps                     # 앱 목록 TUI
cleanmac uninstall -n "App Name"  # 특정 앱 삭제

# 설정
cleanmac config show
cleanmac config set -k key -v value

# 히스토리
cleanmac history                  # 삭제 히스토리 조회
```

### CLI 명령어 (AI/자동화용)
```bash
# 스캔 → JSON 출력
cleanmac scan --format json --out scan.json
cleanmac scan --format json --metadata --out scan.json  # Spotlight 메타데이터 포함

# 계획 수립
cleanmac plan --from scan.json --out plan.json
cleanmac plan --category caches --out plan.json

# 실행
cleanmac apply --plan plan.json --yes --out result.json

# 보고서 생성
cleanmac report --from scan.json --format md --out report.md
cleanmac report --from result.json --format json
```

### JSON 출력 예시
```json
{
  "version": "1.0",
  "timestamp": "2026-02-19T10:30:00Z",
  "categories": [
    {
      "id": "system_caches",
      "name": "System Caches",
      "size_bytes": 2456789012,
      "item_count": 234,
      "items": [
        {
          "path": "/Users/wis/Library/Caches/com.apple.Safari",
          "size_bytes": 12345678,
          "modified": "2026-02-18T10:00:00Z",
          "last_used": "2024-01-15T10:30:00Z",
          "use_count": 0
        }
      ]
    }
  ],
  "total_size_bytes": 5234567890,
  "total_item_count": 567
}
```

### MCP 서버 (AI 연동)

AI(Claude, GPT 등)가 직접 호출할 수 있는 MCP 서버 제공

**Claude Desktop 설정** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "cleanmac": {
      "command": "/usr/local/bin/cleanmac",
      "args": ["mcp"]
    }
  }
}
```

**MCP 도구 목록**:
| 도구 | 설명 |
|------|------|
| `scan_system` | 전체 시스템 스캔 |
| `scan_category` | 특정 카테고리 스캔 (메타데이터 포함) |
| `analyze_disk` | 디스크 사용량 분석 |
| `list_apps` | 설치된 앱 목록 |
| `get_history` | 삭제 히스토리 조회 |
| `preview_clean` | 삭제 미리보기 + CLI 명령어 반환 |

**AI 워크플로우 예시**:
```
1. 사용자: "맥 정리해줘"
2. AI: scan_system 호출
3. AI: "캐시 2.3GB, 로그 500MB 발견"
4. AI: preview_clean 호출 → CLI 명령어 안내
5. 사용자: cleanmac apply --category caches --yes 직접 실행
```

## TUI 키바인딩

### 공통
- `q` - 종료
- `?` - 도움말
- `Esc` - 이전 화면

### 카테고리 선택
- `↑↓` - 이동
- `Space` - 선택/해제
- `r` - 스캔 시작

### 리뷰
- `↑↓` - 아이템 이동
- `←→` - 카테고리 이동
- `Tab` - 카테고리 목록
- `Space` - 아이템 선택
- `s` - 정렬 변경
- `v` - Space Lens
- `Enter` - 삭제 실행

### Space Lens
- `↑↓` - 이동
- `Enter` - 폴더 진입
- `Backspace` - 상위 폴더
- `t` - 스레드 수 변경 (4/8/16)

## 설정

`~/.config/cleanmac/config.toml`

```toml
[scan]
min_size_bytes = 1048576  # 1MB
max_depth = 3
excluded_paths = []

[clean]
dry_run_by_default = true
log_history = true
confirm_before_clean = true

[ui]
show_sizes_in_bytes = false
```

## CleanMyMac과 비교

| 기능 | CleanMyMac | CleanMac CLI |
|------|-----------|--------------|
| 시스템/브라우저/개발 정크 | ✅ | ✅ |
| 앱 완전 삭제 | ✅ | ✅ |
| 디스크 사용량 시각화 | ✅ | ✅ |
| 대용량/중복 파일 | ✅ | ✅ |
| 삭제 히스토리 | ✅ | ✅ |
| 병렬 스캔 | ✅ | ✅ |
| 시작 프로그램 관리 | ✅ | ✅ |
| 시스템 유지보수 | ✅ | ✅ |
| **AI 연동 (MCP)** | ❌ | ✅ |
| **JSON 출력** | ❌ | ✅ |
| GUI | ✅ | ❌ TUI만 |
| 멀웨어 제거 | ✅ | ❌ |
| 실시간 모니터링 | ✅ | ❌ |
| 무료 | ❌ | ✅ |

## 기술 스택

- **언어**: Rust
- **TUI**: ratatui
- **병렬 처리**: rayon, thread pool
- **파일 시스템**: walkdir
- **MCP**: rmcp

## 라이선스

MIT
