# CleanMac AI-Readable CLI + MCP 서버 설계

**작성일**: 2026-02-19  
**상태**: 기획 완료, 구현 대기

---

## 1. 개요

### 1.1 목표
CleanMac을 AI가 쉽게 호출하고 결과를 활용할 수 있도록 설계한다.

### 1.2 핵심 원칙
- **사람은 TUI**: 인터랙티브하게 선택하고 바로 실행
- **AI는 CLI/MCP**: 파일 기반으로 재현 가능한 워크플로우
- **AI는 분석, 사람이 실행**: 삭제 권한은 항상 사용자에게

### 1.3 앱 이름
- **기본**: `cleanmac`
- **짧은 alias**: `cmr` (Homebrew 배포 시)

---

## 2. CLI 구조

### 2.1 서브커맨드

```
cleanmac scan    # 시스템 스캔 → 후보 발견
cleanmac plan    # 스캔 결과 → 삭제 계획 수립
cleanmac apply   # 계획 실행 → 실제 삭제
cleanmac report  # 결과 → 사람용 보고서
```

### 2.2 플래그 규약

| 플래그 | 용도 |
|--------|------|
| `--format json\|human` | 출력 형식 (기본: human) |
| `--out <file>` | 파일로 출력 |
| `--from <file>` | 입력 파일 |
| `--yes` | 비대화형 실행 (확인 생략) |
| `--dry-run` | 실행 없이 미리보기 |
| `--categories <list>` | 특정 카테고리만 처리 |

### 2.3 Exit Code 규약

| 코드 | 의미 |
|------|------|
| 0 | 정상 완료 |
| 1 | 일반 오류 |
| 2 | 정리 후보 있음 (scan 완료, 발견됨) |
| 3 | 부분 실패 (일부 항목 실패) |
| 4 | 권한 부족 |
| 5 | 사용자 취소 |

### 2.4 명령어 예시

**사람 (TUI):**
```bash
cleanmac              # TUI 진입
```

**사람 (CLI):**
```bash
cleanmac scan --format human
cleanmac scan --categories caches,logs
cleanmac apply --yes
```

**AI (파일 기반):**
```bash
cleanmac scan --format json --out scan.json
cleanmac plan --from scan.json --out plan.json
# → AI가 plan.json 분석 후 사용자에게 권장
cleanmac apply --plan plan.json --yes --log json --out apply.json
cleanmac report --from apply.json --format md --out report.md
```

---

## 3. MCP 서버

### 3.1 실행 방식
```bash
cleanmac --mcp    # MCP 서버 모드 (stdio 통신)
```

### 3.2 MCP 도구 목록

#### 읽기/분석 도구 (기본 제공)

| 도구 | 설명 | 입력 | 출력 |
|------|------|------|------|
| `scan_system` | 전체 시스템 스캔 | categories? | ScanResult |
| `scan_category` | 특정 카테고리 스캔 | category | CategoryResult |
| `analyze_disk` | 디스크 사용량 분석 | path, depth? | DiskAnalysis |
| `list_apps` | 설치된 앱 목록 | - | AppList |
| `get_history` | 삭제 히스토리 | limit? | HistoryList |
| `preview_clean` | 삭제 미리보기 | categories | PreviewResult |

#### 실행 도구 (config에서 명시적 허용 시만)

| 도구 | 설명 | 입력 | 출력 |
|------|------|------|------|
| `execute_clean` | 실제 삭제 실행 | plan_id | ExecutionResult |

### 3.3 MCP 도구 상세 정의

#### scan_system
```json
{
  "name": "scan_system",
  "input": {
    "categories": ["system_caches", "logs"]  // 선택적, 없으면 전체
  },
  "output": {
    "categories": [
      {
        "id": "system_caches",
        "name": "System Caches",
        "size_bytes": 2456789012,
        "item_count": 234,
        "description": "시스템 및 앱 캐시 파일"
      }
    ],
    "total_size": 5234567890,
    "total_items": 567,
    "scan_duration_ms": 1234
  }
}
```

#### preview_clean
```json
{
  "name": "preview_clean",
  "input": {
    "categories": ["system_caches"]
  },
  "output": {
    "items": [
      {
        "path": "/Library/Caches/com.apple.Safari",
        "size": 12345678,
        "last_used": "2024-01-15T10:30:00Z",
        "use_count": 0
      }
    ],
    "total_size": 2456789012,
    "cli_command": "cleanmac apply --plan plan_abc123 --yes",
    "warnings": [
      "Safari 캐시 삭제 시 웹 로그인 필요할 수 있음"
    ]
  }
}
```

#### analyze_disk
```json
{
  "name": "analyze_disk",
  "input": {
    "path": "/Users/wis",
    "depth": 2
  },
  "output": {
    "path": "/Users/wis",
    "total_size": 123456789012,
    "children": [
      {
        "name": "Library",
        "size": 45678901234,
        "percent": 37.0
      },
      {
        "name": "Documents",
        "size": 12345678901,
        "percent": 10.0
      }
    ]
  }
}
```

### 3.4 AI 워크플로우 예시

```
1. 사용자: "맥 정리해줘"
2. AI: scan_system 호출
3. AI: 결과 분석 → "캐시 2.3GB, 로그 500MB 발견"
4. AI: preview_clean 호출 → 삭제할 항목 상세 확인
5. AI: "다음 명령어를 실행하면 됩니다: cleanmac apply --plan xxx --yes"
6. 사용자: 직접 실행
7. AI: get_history로 결과 확인
```

---

## 4. 데이터 구조

### 4.1 ScanResult (scan.json)
```json
{
  "version": "1.0",
  "timestamp": "2026-02-19T10:30:00Z",
  "categories": [
    {
      "id": "system_caches",
      "name": "System Caches",
      "description": "시스템 및 앱 캐시 파일",
      "size_bytes": 2456789012,
      "item_count": 234,
      "items": [
        {
          "path": "/Library/Caches/com.apple.Safari",
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

### 4.2 PlanResult (plan.json)
```json
{
  "version": "1.0",
  "timestamp": "2026-02-19T10:35:00Z",
  "scan_file": "scan.json",
  "categories": [
    {
      "id": "system_caches",
      "action": "delete",
      "items": [
        {
          "path": "/Library/Caches/com.apple.Safari",
          "size_bytes": 12345678
        }
      ]
    }
  ],
  "total_size_bytes": 2456789012,
  "warnings": [
    "Safari 캐시 삭제 시 로그인 필요할 수 있음"
  ]
}
```

### 4.3 ExecutionResult (apply.json)
```json
{
  "version": "1.0",
  "timestamp": "2026-02-19T10:40:00Z",
  "plan_file": "plan.json",
  "status": "success",
  "categories": [
    {
      "id": "system_caches",
      "status": "success",
      "deleted_count": 200,
      "deleted_size_bytes": 2000000000,
      "failed_count": 0
    }
  ],
  "total_deleted_size": 2000000000,
  "duration_ms": 5432
}
```

---

## 5. 메타데이터 수집

### 5.1 수집 항목

| 항목 | 수집 방법 | 신뢰도 |
|------|----------|--------|
| 파일 크기 | `st_size` | ✅ |
| 수정일 | `st_mtime` | ✅ |
| 접근일 | `st_atime` | ⚠️ 백업/인덱싱 영향 |
| 마지막 사용일 | `mdls kMDItemLastUsedDate` | ✅ |
| 사용 횟수 | `mdls kMDItemUseCount` | ✅ |

### 5.2 Spotlight 메타데이터 활용

```bash
# 파일의 마지막 사용일과 사용 횟수 조회
mdls -name kMDItemLastUsedDate -name kMDItemUseCount /path/to/file
```

```rust
// Rust 구현 예시
fn get_spotlight_metadata(path: &Path) -> Option<FileMetadata> {
    let output = Command::new("mdls")
        .args(["-name", "kMDItemLastUsedDate", "-name", "kMDItemUseCount"])
        .arg(path)
        .output()
        .ok()?;
    
    // 파싱 로직...
}
```

### 5.3 AI 판단 근거 예시

```
📁 /Library/Caches/com.apple.OldApp
   크기: 1.2GB
   마지막 사용: 2022-03-15 (4년 전)
   사용 횟수: 0
   
   → AI: "4년간 사용하지 않은 앱 캐시입니다. 삭제 권장"
```

---

## 6. TUI vs CLI/MCP 분리

### 6.1 사용자 경험 분리

| 대상 | 인터페이스 | 워크플로우 |
|------|-----------|-----------|
| 일반 사용자 | TUI | 진입 → 스페이스로 선택 → 엔터로 실행 |
| 파워유저 | CLI | scan → plan → apply |
| AI | MCP | scan_system → preview_clean → CLI 명령어 안내 |

### 6.2 TUI 기능 (변경 없음)
- 카테고리별 스캔
- 스페이스로 항목 선택/해제
- 엔터로 즉시 정리
- 히스토리 확인

### 6.3 CLI/MCP 기능 (새로 추가)
- JSON 출력/입력
- 파일 기반 워크플로우
- Exit code로 결과 전달
- MCP 서버 모드

---

## 7. 구현 계획

### Phase 1: CLI 구조 개선
- [ ] 서브커맨드 구조 변경 (scan, plan, apply, report)
- [ ] `--format json/human` 플래그 추가
- [ ] `--from`, `--out` 파일 입출력
- [ ] Exit code 규약 구현

### Phase 2: 메타데이터 수집
- [ ] Spotlight 연동 (mdls)
- [ ] last_used, use_count 수집
- [ ] ScanResult에 메타데이터 포함

### Phase 3: MCP 서버
- [ ] MCP 프로토콜 구현 (rmcp crate)
- [ ] 도구 6개 구현 (scan_system, preview_clean 등)
- [ ] Claude Desktop 설정 문서화

### Phase 4: 문서화
- [ ] README 업데이트
- [ ] AI 사용 가이드
- [ ] MCP 설정 예시

---

## 8. 기술 스택

| 항목 | 기술 |
|------|------|
| MCP 서버 | `rmcp` crate (Rust MCP 구현) |
| JSON 스키마 | `serde_json`, `schemars` |
| CLI | `clap` (기존) |
| Spotlight | `std::process::Command` + mdls |

---

## 9. 참고 자료

- [MCP (Model Context Protocol)](https://modelcontextprotocol.io/)
- [rmcp - Rust MCP Implementation](https://github.com/anthropics/rmcp)
- [Spotlight Metadata Attributes](https://developer.apple.com/documentation/coreservices/mditem)

---

**다음 단계**: Phase 1 구현 시작
