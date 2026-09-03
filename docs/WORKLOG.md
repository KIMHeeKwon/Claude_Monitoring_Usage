# WORKLOG — Claude_Monitoring_Usage

## 2026-09-03 — 설계 착수 (세션 1)

**목표**: "Claude 사용량 + CPU·메모리·GPU를 보여 주는 상시 표시 창"의 구조 설계. 코드 없음.

**결정사항**
- 등급 L2. 게이트 G1(사각지대) → G4.2(선행 조사) → G2(질문) → G4.5(계획). **G3(화면 시안)은 사용자와
  별도 세션에서 함께 진행**하기로 사용자가 지시 — 초벌 스케치만 남김.
- 저장소를 `KIMHeeKwon/Claude_Monitoring_Usage`(개인 계정)에 연결. 로컬 커밋 이메일 hkkim79@gmail.com.
- 표시 항목에 CPU·메모리·GPU 추가 (사용자 요청, 세션 중).
- P0 미결 4건(Q1~Q4)과 추천안을 [[DECISIONS]]에 기록. 추천안 전제로 설계.

**산출물**
- [[ARCHITECTURE]] v0.1 — 요구·전제·구조·데이터 계약·소스별 설계·실패 상태·배포·마일스톤
- [[DECISIONS]] — P0 질문 4건, P1 2건, 확정 5건
- [[PRIOR-ART-SURVEY]] — 기존 도구·데이터 소스·프레임워크·시스템 지표 조사 (서브에이전트)
- `docs/mockups/ui-directions.html` — 초벌 시안 4방향 (확정본 아님)
- `CLAUDE.md`, `.gitignore`

**실측 (2026-09-03, 이 PC 1대)**
- RTX 4090 24GB 별도 VRAM, `nvidia-smi` 동작, PDH `GPU Engine` 카운터 존재
- Node 24.19 · Python 3.12 있음, Rust 툴체인 없음 (Tauri 채택 시 rustup 설치 필요)
- Claude Code 기록 `.jsonl`의 `message.usage`에 토큰 필드 존재 (1건 확인)

**조사가 바꾼 설계 (2026-09-03, [[PRIOR-ART-SURVEY]] §1)**
- 한도 소스를 단일(비공식 조회)에서 **2계층**으로 변경: 1순위 statusline 훅(공식·정책 안전), 2순위 `/api/oauth/usage`
  (비공식, 기본 꺼짐 옵트인 + 약관 고지). 근거: Anthropic 2026-02-19 약관 명문화.
- macOS 서명 없는 배포 절차 수정: "우클릭 → 열기"는 Sequoia부터 불가 → ad-hoc 서명 + "그래도 열기" 안내.
- 조회 주기 60초 → 180초, `User-Agent: claude-code/<버전>` 필수, 상태에 `stale`·`no_source`·`source` 추가.

**현재 진행도**: 설계 문서 v0.1 완료(조사 반영). 코드 0줄.

**사용자 답변으로 확정 (2026-09-03, 같은 날 후반)**
- D6 구독 한도 정의(Pro 감안) · D7 Max+Pro · D8 **Tauri v2** · D9 statusline 기본 + **저장된 토큰 읽기 옵트인**.
  P0 미결 0건. "Claude Code"는 CLI를 뜻함을 확인. 사용자 `settings.json`에 기존 statusline(orca, Mac 경로)이
  있고 2대 동기화됨 → 훅은 감싸기 + OS 중립 경로 필수 (ARCHITECTURE §5.1 실측 기재).

## 2026-09-03 — M0 관통 골격 (세션 1 후반)

**목표**: Tauri v2로 항상-위 창 + 트레이 + CPU/메모리 표시, Windows 설치 파일까지 관통.

**결정사항 / 이탈**: NSIS currentUser 설치 파일 채택(관리자 권한 불필요), 프론트엔드 빌드 도구 없음(정적 HTML +
`withGlobalTauri`). 상세는 `implementation-notes.md` Deviations.

**산출물**
- `src-tauri/` (lib.rs 트레이·창 이벤트, sysmon.rs 1초 샘플링 → `sys:update`), `src/index.html` 임시 화면, `README.md`
- 설치 파일 `src-tauri/target/release/bundle/nsis/Claude Usage_0.0.1_x64-setup.exe` — **1.9MB** (실행 파일 8.9MB)

**실측 (2026-09-03, 이 PC)**
- 환경 구축: rustup(winget) + stable 1.98 + MSVC Build Tools 14.44 + Windows SDK 10.0.26100. Build Tools 약 15분
- `cargo build` debug 2분 42초, 경고 0. debug·release 실행 파일 모두 `timeout`으로 6~8초 상주 확인 (크래시 없음)
- **화면에 숫자가 그려지는지는 미확인** — Claude가 화면을 볼 수 없어 사용자 육안 확인 대기

**현재 진행도**: M0 산출물 완료, 육안 검증 대기. 다른 PC 설치 검증(M0 통과 기준)은 미실시.

**남은 미해결**
- 다른 Windows PC 설치 검증 (M0 통과 기준)

## 2026-09-03 — 디자인 인계본 반영 (세션 1 마지막)

**목표**: `Design/design_handoff_usage_monitor_widget/`의 확정 시안을 창에 구현. 사용자 지시로 **가로형 7종 전부**.

**결정사항**: D10(7종 전부 + 설정 선택), D11(레이아웃 전환 시 창 크기 자동 변경). M0 창은 사용자 육안 확인 통과.

**산출물**
- `src/styles.css` — Industry 토큰을 CSS 변수 두 세트(다크/라이트)로. 블루프린트 프레임 + 네 귀 등록 마크,
  상태 3단계(75%/90%)와 5H 숫자 펄스, 레이아웃 7종 기하
- `src/app.js` — 데이터 계약만 받아 그리는 렌더러(약 220줄). 40표본 링 버퍼, 계기 호 길이 실측값,
  status 7종 문구, Opus·GPU 부재 시 행 숨김, 예시 값 모드
- `src-tauri/src/settings.rs`(설정·레이아웃 크기 표), `src-tauri/src/menu.rs`(우클릭·트레이 공용 메뉴)
- 글꼴 8종 번들(`src/fonts/`, 1.8MB) — 상주 앱이 네트워크로 글꼴을 받지 않게

**검증 (2026-09-03)**
- `cargo build` 통과(경고 0), 실행 파일 7초 상주 확인
- 브라우저 미리보기로 **7종 × 다크·라이트 전부 렌더 확인**(Claude 육안). 두 곳 수정: 등록 마크가
  `overflow:hidden`에 잘리던 것, 시스템 값의 `%` 누락
- **앱 창에서의 육안 확인은 사용자 대기** — 특히 투명 창에서 등록 마크가 보이는지, 메뉴로 레이아웃을
  바꿀 때 창 크기가 맞는지

**남은 미해결**
- 위 육안 확인 2건, 다른 PC 설치 검증

## 2026-09-03 — M1: statusline 훅으로 한도 표시 (세션 1 연장)

**목표**: 공식 경로(Claude Code statusline JSON)로 5시간·주간·Opus 한도를 창에 띄운다.

**산출물**
- `src-tauri/hook/hook.sh` — stdin의 statusline JSON을 `status.json`에 저장하고, **기존 statusline 명령을
  그대로 이어서 실행**한다(감싸기). 사용자의 기존 표시가 유지된다.
- `src-tauri/src/usage.rs` — 훅 설치·제거(설정 백업 + 원래 명령 복원), 2초 폴링, 계약 정규화, 상태 판정.
  `CLAUDE_CONFIG_DIR`를 존중한다.
- 메뉴에 "Claude 한도 연결 (statusline 훅)" 토글. 사용자 설정 파일을 고치는 동작이므로 명시적 선택.
- `src/app.js` — 상태 문구에 `waiting` 추가, 값이 없을 때 `%` 기호도 숨김.

**검증 (2026-09-03)**
- **자동 테스트 2건 통과**: ① 정규화 — Pro(5시간 창만)·Max(세 창)·`rate_limits` 없음·형식 변경·JSON 아님을
  각각 올바른 상태로 판정. ② 감싸기 — 기존 statusline 보존, 다른 설정 불변, 백업 생성, 두 번 설치해도 원래
  명령 유지, 제거 시 원상 복구.
- 훅 스크립트를 표본 JSON으로 직접 실행해 `status.json` 생성 확인.
- 화면: 상태 6종(정상·주의·위험·끊김·대기·미연결)을 미리보기로 확인. 문구·색·Opus 행 숨김 모두 정상.

**실측 성공 (2026-09-03, Claude Code 2.1.259 · 사용자 터미널 세션 1회)**
- 훅이 실제로 실행되어 `rate_limits`가 들어왔다: 5시간 33%, 주간 8%. `resets_at`은 epoch 초.
- **`seven_day_opus`는 오지 않았다** — 창 부재 허용 설계(ARCHITECTURE §4)가 실제로 필요함이 확인됐다.
- 첫 `claude` 실행 직후에는 파일이 없었고, 세션이 응답을 한 번 낸 뒤 생겼다(공식 문서와 일치).
  사용자가 "반응 없음"을 본 것은 이 시차 때문이다 → README에 "값이 들어오기까지 한 번의 대화가 필요"를 명시.
- 부수 발견: 같은 파일에 `cost`·`context_window`·`model`·`effort`가 있다. **비용 표시를 로컬 기록 파싱 없이**
  할 수 있는 경로이므로 M5 계획을 바꿀 수 있다 (단 세션 누계).

**미검증 (사용자 확인 필요)**
- 앱 창에 그 값이 실제로 그려지는지 (파일은 확인, 화면은 육안 대기)
- 이 PC의 `settings.json`은 **실측을 위해 이미 훅으로 감싸 두었다**. 백업은
  `~/.claude/settings.json.bak-20260903-101508`. 되돌리려면 앱 메뉴에서 연결을 끄면 된다.

**남은 미해결**
- 위 미검증 2건, 다른 PC 설치 검증
- M1b(옵트인 조회), M2(GPU), M3(macOS 빌드)
- statusline 훅이 이 계정에서 `rate_limits`를 실제로 내보내는지 — 이 PC에서 훅 1회 실행으로 확정 (M1 첫 작업)
- `/api/oauth/usage` 응답 형식의 직접 실측 (2차 자료 기반 — 옵트인 구현 전 curl 1회)
- macOS GPU(IOKit) 경로 미검증 — MacBook에서 M2 때 확인
- 저장소를 공개로 둘지 (SignPath 무료 서명의 전제) — 사용자 결정

**다음 단계**
1. 사용자 답변 반영 → DECISIONS 확정 → 화면 시안 공동 세션
2. M0 관통 골격 (항상-위 창 + CPU/메모리 + Windows `.msi`)
