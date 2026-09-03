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

**남은 미해결**
- statusline 훅이 이 계정에서 `rate_limits`를 실제로 내보내는지 — 이 PC에서 훅 1회 실행으로 확정 (M1 첫 작업)
- `/api/oauth/usage` 응답 형식의 직접 실측 (2차 자료 기반 — 옵트인 구현 전 curl 1회)
- macOS GPU(IOKit) 경로 미검증 — MacBook에서 M2 때 확인
- 저장소를 공개로 둘지 (SignPath 무료 서명의 전제) — 사용자 결정

**다음 단계**
1. 사용자 답변 반영 → DECISIONS 확정 → 화면 시안 공동 세션
2. M0 관통 골격 (항상-위 창 + CPU/메모리 + Windows `.msi`)
