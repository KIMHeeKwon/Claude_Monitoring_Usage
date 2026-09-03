# 선행 도구·데이터 소스·프레임워크 조사 (G4.2, 2026-09-03)

> 조사 주체: 서브에이전트(웹 검색·README 열람 106회, 2026-09-03). 직접 확인하지 못한 항목은 **미검증**으로
> 표시했다. 이 문서는 [[ARCHITECTURE]] §2·§5·§7과 [[DECISIONS]] Q3·Q4의 근거다. 부록에 원문(영문) 보고를
> 그대로 실었다 — URL과 세부 수치는 부록이 원천이다.

## 1. 설계를 바꾼 발견 5가지

1. **로그인 토큰을 다른 도구에서 쓰는 것은 약관 위반이다 (2026-02-19 명문화).** Anthropic 법무 페이지:
   "Claude Free/Pro/Max 계정으로 얻은 OAuth 토큰을 다른 제품·도구·서비스에서 쓰는 것은 소비자 약관 위반".
   읽기 전용 조회와 추론을 구분하지 않는다. 실제 차단은 추론 도구(OpenClaw 등)에만 내려졌고 사용량 조회
   도구들은 2026-09-01까지 계속 배포 중이지만, **문서상 금지 · 실제로는 묵인** 상태다. 동료에게 나눠 주는
   앱이라면 이 위험을 사용자에게 알리고 선택하게 해야 한다. (인용: The Register 2026-02-20, 부록 B.1.2)
2. **공식 문서화된 경로가 하나 있다: Claude Code statusline JSON.** `rate_limits.five_hour.used_percentage`,
   `rate_limits.seven_day.used_percentage`, `resets_at`(epoch 초)가 `/usage`와 같은 값으로 제공된다. 단, Claude Code
   세션이 떠 있고 첫 응답을 받은 뒤에만 나오며, Pro/Max 한정이다. VS Code 확장(bartosz-warzocha)과
   Claude-Code-Usage-Monitor v4가 이 방식을 쓴다. (인용: code.claude.com/docs/en/statusline, 부록 B.4)
3. **비공식 조회 주소의 운용 조건이 까다롭다.** `GET api.anthropic.com/api/oauth/usage`는 `User-Agent:
   claude-code/<버전>`이 없으면 429가 지속되고, 토큰 단위로 제한되며, 권장 주기는 180초 이상이다. 토큰은 약 60분마다
   만료되어 Claude Code가 갱신해 줘야 한다. 응답 필드는 `five_hour`, `seven_day`, `seven_day_opus`,
   `seven_day_sonnet`, `extra_usage`. (인용: Claude-Code-Usage-Monitor issue #202, 부록 B.1.1)
4. **macOS 키체인 읽기는 앞으로 막힐 수 있다.** 현재는 ACL 없이 저장돼 어떤 프로세스든 읽을 수 있지만(Silverfort
   2026-07-24), Anthropic이 접근 제어 강화를 예고했다. Windows는 `%USERPROFILE%\.claude\.credentials.json` 평문
   파일이고 Credential Manager를 쓰지 않는다 (공식 문서 확인). (부록 B.2)
5. **서명 없는 macOS 배포는 이제 "우클릭 → 열기"로 안 된다.** Sequoia부터 시스템 설정 → 개인정보 보호 및 보안 →
   "그래도 열기" + 관리자 암호가 필요하고, 서명 없는 Tauri 빌드는 Apple Silicon에서 "손상됨"으로 뜬다(ad-hoc 서명으로
   회피 가능). Windows는 EV 인증서도 SmartScreen을 즉시 통과시키지 못하며, 공개 오픈소스라면 SignPath Foundation의
   무료 서명을 받을 수 있다. (부록 C)

## 2. 데이터 소스 후보표 (실행 가능성 열 포함)

| # | 소스 | 주는 값 | 실행 가능성 (개인 Pro/Max 구독자) | 정책·기술 위험 |
|---|---|---|---|---|
| S-A | **statusline JSON** (공식) | 5시간·주간 사용률 %, 초기화 시각 | **높음, 정책 안전.** 사용자의 `settings.json`에 statusline 훅 설치 필요 | Claude Code 세션이 없으면 갱신 안 됨 → 마지막 값 + 경과 시간 표시로 보완 |
| S-B | `/api/oauth/usage` (비공식) + Claude Code 토큰 | 위와 동일 + Opus/Sonnet 주간, 추가 사용 크레딧 | 기술적으로 높음. **정책상 약관 위반 문구에 해당** | 429·형식 변경·키체인 ACL 강화·토큰 만료. 옵트인 + 고지 필요 |
| S-C | 로컬 기록 `.jsonl` 집계 | 토큰 수·추정 비용(LiteLLM 단가) | 높음, 정책 안전, 오프라인 | Claude Code 사용분만. 한도 %는 산출 불가(한도 자체가 비공개) |
| S-D | claude.ai 세션 쿠키 → 내부 API | 설정 페이지와 동일한 값 | 중간 (쿠키 추출 필요) | 비공식, 약관 위험, 쿠키 회전. **기각** |
| S-E | Admin API `usage_report` | 조직 단위 | **해당 없음** (개인 구독에는 조직 관리자 키가 없음) | — |
| S-F | Claude Desktop 앱 로컬 파일 | — | 읽을 수 있는 사용량 파일 없음 (미검증 부정) | — |

## 3. 기존 도구 요약 (전체 표는 부록 A)

| 도구 | 플랫폼 | 스택 | 소스 | 배포·서명 | 우리에게 주는 교훈 |
|---|---|---|---|---|---|
| CodexBar (steipete) 20.9k★ | mac | Swift | OAuth/쿠키/CLI PTY 순 폴백 | brew, 서명·공증 | **소스 계층화 + 폴백 순서 명시** |
| Win-CodexBar (nesszer) 1k★ | Win | **Tauri + React + Rust** | 쿠키 우선, OAuth 폴백 | winget, 미서명(SignPath 대기) | Tauri 트레이 앱의 직접 선례 |
| aqua5230/usage 306★ | **Win + mac** | Python + WebView | **JSONL만** (API 호출 없음) | brew cask, SignPath 서명 | 정책 안전 노선의 실례, 두 OS 한 코드베이스 |
| Claude-Code-Usage-Monitor 8.7k★ | 터미널 | Python | JSONL + **statusline 캡처** + OAuth(실험) | PyPI | statusline 캡처 구현 참고 |
| jens-duttke/usage-monitor-for-claude | Win | Python + pystray | OAuth, 만료 시 `claude update`로 갱신 유도 | winget, 12.5MB EXE | **토큰 갱신을 앱이 직접 하지 않는** 방식 |
| xinggaoya/system-monitor | Win/mac/Linux | Tauri 2.2 + sysinfo + nvml-wrapper | 시스템 지표 | 6~8MB | **CPU·메모리·GPU를 Tauri 안에서 읽는 선례** |
| Stats (exelban) 41.6k★ | mac | Swift | 시스템 지표 | brew | 메뉴바 시스템 모니터의 기준점 |

두 플랫폼 모두에서 서명된 무료 도구는 없었다.

## 4. 프레임워크 후보표

| 프레임워크 | 크기 | 트레이·항상-위·투명 | 시스템 지표 수집 | 동료 설치 | 판정 |
|---|---|---|---|---|---|
| **Tauri v2** | 6~8MB (실례) | 지원. mac 투명창은 `macOSPrivateApi` 필요 | sysinfo·nvml-wrapper·PDH·IOKit **전부 프로세스 내부에서** | .msi/.exe, .dmg. WebView2는 Win10/11 기본 | **채택 추천** — 상주 앱에 맞고 지표 수집이 가장 깔끔 |
| Electron | 85~200MB, 유휴 RSS 150MB+ | 지원 | `systeminformation`이 Windows에서 PowerShell을 띄움, GPU는 nvidia-smi 자식 프로세스 | 크지만 가능 | 상시 표시 앱에는 무거움 |
| Python (pystray/pywebview + PyInstaller) | 12~40MB | 가능 | psutil·nvidia-ml-py·win32pdh, mac은 `ioreg` 자식 프로세스 | 가능하나 AV·Gatekeeper 오탐 잦음 | 빠르지만 배포 품질이 낮음 |
| Avalonia (.NET) | 18~58MB | 지원 | Win은 쉬움, mac은 수동 interop | 가능 | mac 쪽 노동이 큼 |
| Swift + C# 이중 네이티브 | 최소 | 최상 | 최상 | 가능 | 코드베이스 2개 — R6 위반 |
| 웹 페이지만 | 0 | 불가 | 불가 | URL | 단독으로는 불가 |

## 5. 시스템 지표 수집 방법 (Tauri 기준)

| 지표 | Windows | macOS | 비고 |
|---|---|---|---|
| CPU·메모리 | `sysinfo` 0.39 (`refresh_cpu_usage` 2회 간격 필요) | 동일 | 권한 불필요 |
| GPU (NVIDIA) | `nvml-wrapper` 0.13 — 런타임에 `nvml.dll` 로드, 없으면 우아하게 실패 | 해당 없음 | 사용률·메모리 둘 다 |
| GPU (제조사 무관) | PDH `\GPU Engine(*)\Utilization Percentage` — 인스턴스 열거 후 엔진별 max (작업 관리자 방식). `precord` 크레이트가 구현 예 | — | 인스턴스 수백 개 → 쿼리 핸들을 열어 두고 재사용 |
| GPU (Apple Silicon) | — | IOKit `IOAccelerator`/`AGXAccelerator`의 `PerformanceStatistics` → `Device Utilization %` (공개 API, sudo 불필요). 대안: IOReport 기반 `macmon` 크레이트(비공개 API, OS 업데이트에 취약) | 유휴 시 비정상 고값 보고 사례 있음(AirStats) |
| GPU (Intel Mac) | — | 같은 IOKit 경로 (`IntelAccelerator`, `AMDRadeonX6000_*`) | 미검증 |

원칙: 1~2초 주기, 핸들(PDH·NVML·IOKit) 상시 유지, 틱마다 자식 프로세스 금지.

## 6. 남은 미검증 항목 (구현 전 실측 대상)

- `/api/oauth/usage` 응답 형식 — 2차 자료만 있음. 이 PC에서 curl 1회로 확정 (S-B를 쓸 경우)
- statusline JSON이 실제로 `rate_limits`를 이 계정에서 내보내는지 — 이 PC에서 훅 1회 실행으로 확정
- macOS IOKit GPU 값의 신뢰성 — MacBook에서 M2 단계 확인
- Azure Artifact Signing의 한국 개인 자격 — 개인은 US/CA 한정으로 읽히나 상충 자료 있음

---

## 부록 — 원문 조사 보고 (영문, 서브에이전트 산출 2026-09-03)

### A. Existing tools that show Claude usage on the desktop

| Tool | Shows | Data source | Stack | Win / mac | Distribution | Last activity | License | URL |
|---|---|---|---|---|---|---|---|---|
| ccusage (ryoppippi) | Daily/monthly/session token counts + USD cost, "5-hour blocks" (heuristic), `ccusage statusline` | Local JSONL (`~/.claude/projects`, `~/.config/claude/projects`), LiteLLM pricing | TypeScript CLI | Both + Linux (CLI only) | npm/npx, Nix | v20.0.20, 2026-08-15; 18.3k stars | MIT | https://github.com/ryoppippi/ccusage |
| Claude-Code-Usage-Monitor (Maciek-roboblog) | 5-h window %, weekly %, tokens, cost, burn rate | JSONL + official statusline `rate_limits` capture + opt-in "experimental Anthropic OAuth usage API" | Python 3.9+, Rich TUI | Both + Linux (terminal) | PyPI `uv tool install claude-monitor` | v4.0.0, 2026-06-27; 8.7k stars | MIT | https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor |
| CodexBar (steipete) | Per-provider session/weekly/monthly windows, reset countdowns | Claude: OAuth API, browser cookies, or CLI PTY fallback | Swift 6.2, macOS 14+ | mac only | brew cask; signed + notarized; Sparkle | 0.56.3, 2026-09-01; 20.9k stars | MIT | https://github.com/steipete/CodexBar |
| Win-CodexBar (nesszer) | Same as CodexBar, tray-first | Claude: browser cookies preferred, OAuth/CLI fallback; DPAPI secrets | Tauri + React + Rust | Win only | winget, installer + portable; unsigned (SignPath pending) | v0.55.0, 2026-08-24; 1k+ stars | MIT | https://github.com/nesszer/Win-CodexBar |
| CodexBar-Win (babakarto) | Session, weekly, reset, API cost | `claude` CLI via PTY `/usage` → `.credentials.json` OAuth → cookies → JSONL | Python 3.10 + customtkinter | Win only | exe / pip | 101 stars; date UNVERIFIED | MIT | https://github.com/babakarto/CodexBar-Win |
| ClaudeBar (tddworks) | Session, weekly, per-model % | via `claude` CLI | Swift 6.2 / SwiftUI, macOS 15+ | mac only | brew cask; signed + notarized | 1.5k stars | MIT | https://github.com/tddworks/ClaudeBar |
| Claude-Usage-Tracker (hamed-elfayome) | 5-h, weekly, Opus, API console cost | `claude.ai/api/organizations/{org}/usage` (cookie), embedded sign-in, or Claude Code OAuth | Swift/SwiftUI, macOS 14+ | mac only | Homebrew, Nix; signed + notarized | v3.3.0, 2026-08-29; 3.4k stars | MIT | https://github.com/hamed-elfayome/Claude-Usage-Tracker |
| usage (aqua5230) | Claude Code/Codex/… quota, burn rate, cost | Passive JSONL only ("never calls Anthropic APIs") | Python 3.13 + web UI; mac menu bar, Win WebView2 tray | Both + Linux CLI | brew cask; SignPath-signed download | 306 stars | AGPL-3.0 | https://github.com/aqua5230/usage |
| usage-monitor-for-claude (jens-duttke) | Session + weekly bars in tray icon, alerts | `.credentials.json` → api.anthropic.com; runs `claude update` to refresh; adaptive polling | Python 3.10 (pystray, pywebview) | Win 10/11 (~12.5 MB EXE) | winget; portable EXE | 270 stars | MIT | https://github.com/jens-duttke/usage-monitor-for-claude |
| Claude-Usage-Tracker-Windows (xMazaki) | 5-h, weekly, credits | Manual `sessionKey` paste; AES-256-GCM store | Electron 28 + React | Win only | Portable .exe, unsigned | 1 star | MIT | https://github.com/xMazaki/Claude-Usage-Tracker-Windows |
| claude-usage-tray (ksmaster03) | Tray gauge | `.credentials.json` → `/api/oauth/usage` | Python + tkinter, PyInstaller | Win | Setup.exe; not signed (SmartScreen documented) | 0 stars | MIT | https://github.com/ksmaster03/claude-usage-tray |
| claude-usage-monitor (narendrancm) | 5-h/weekly, history | Claims Windows Credential Manager (UNVERIFIED, contradicts official docs) | Python + pywebview + SQLite | Win | .exe | UNVERIFIED | MIT | https://github.com/narendrancm/claude-usage-monitor |
| ClaudeMeter (eddmann) | 5-h, 7-day, Sonnet | claude.ai web API + `sessionKey`; README: may violate ToS | Swift, macOS 14+ | mac | brew tap; signed + notarized | 161 stars | MIT | https://github.com/eddmann/ClaudeMeter |
| claude-limits (kacharhin) | "5h 6% · 7d 9%" menu bar | Keychain → `/api/oauth/usage` | Swift, macOS 13+ | mac | source | 0 stars | MIT | https://github.com/kacharhin/claude-limits |
| claude-usage (linuxlewis) | Utilization + reset, multi-account | claude.ai cookies | Swift | mac 13+ | ZIP; unsigned | 23 stars | UNVERIFIED | https://github.com/linuxlewis/claude-usage |
| Usage4Claude (f-is-h) | 5-h, 7-day, extra, per-model, multi-account | Browser login | mac menu bar (UNVERIFIED stack) | mac | Releases | UNVERIFIED | UNVERIFIED | https://github.com/f-is-h/Usage4Claude |
| claude-usage Rust crate | Library: 5-h/7-day utilization | Keychain / `.credentials.json` → `/api/oauth/usage` | Rust + napi-rs | mac + Linux only (no Windows) | crates.io | v0.2.3, 2026-01-28 | MIT/Apache-2.0 | https://lib.rs/crates/claude-usage |
| VS Code: Claude Code Status Bar Monitor (bartosz-warzocha) | "Reset: 02:13:20 \| 5h: 6% \| 7d: 35% \| C: $31.34" | Statusline script mirrors Claude Code statusline JSON (`rate_limits`) to a file; JSONL for cost | VS Code ext | Both | Marketplace, 2,227 installs | 2026-08-31 | MIT | https://marketplace.visualstudio.com/items?itemName=bartosz-warzocha.claude-statusbar |
| Other VS Code extensions | 5-h/7-day bars | Mostly OAuth endpoint (UNVERIFIED) | — | Both | Marketplace | — | — | https://marketplace.visualstudio.com/items?itemName=dreyka-oas.claude-ratelimit-statusbar , https://marketplace.visualstudio.com/items?itemName=AndJae.claudecode-usage , https://github.com/gagar1n/vscode-claude-statusline |
| Other mac menu-bar clones (UNVERIFIED) | 5-h/weekly | Keychain → OAuth endpoint | Swift | mac | source/DMG | — | — | https://github.com/jpuritz/Claudar , https://github.com/figueiredouc/claude-limits , https://github.com/kemalasliyuksek/ClaudeBar , https://github.com/kiminmonaco/ClaudeBar , https://github.com/ArrivaRUS/claude-codex-limits |

Observations: ecosystem split into mac Swift menu-bar apps (CodexBar dominates) and Windows tray ports in Python or Tauri (Win-CodexBar largest). Only aqua5230/usage ships one codebase for both, and it deliberately avoids the API (JSONL only). No free tool is signed on both platforms.

### B. Data sources

#### B.1 Table

| # | Source | Yields | Feasibility (individual Pro/Max) | Risk |
|---|---|---|---|---|
| 1 | `GET https://api.anthropic.com/api/oauth/usage` (OAuth subscription token) | `five_hour`, `seven_day`, `seven_day_opus`, `seven_day_sonnet` (null when unused), `extra_usage{is_enabled, monthly_limit, used_credits, utilization}`; each window `{utilization: 0–100, resets_at: ISO-8601}` | High technically — same numbers as `/usage` and claude.ai settings; pooled across chat/Desktop/Code | Undocumented; policy risk (B.1.2); persistent 429 without `User-Agent: claude-code/<ver>`; per-token rate limiting; token expires ~60 min, Claude Code refreshes; Keychain ACL hardening on Anthropic's roadmap |
| 2 | Claude Code credential store (Keychain / `.credentials.json`) | Bearer token for #1 | High on Windows (plain JSON); high on mac today | Reading another app's token is what the Feb-2026 clarification calls unauthorized; mac may lock the item later |
| 3 | Local JSONL (`~/.claude/projects/**/*.jsonl`) | Tokens per message/model, USD via LiteLLM, 5-h "blocks" | High and policy-safe | Claude Code only; no true % of limit (limit unpublished, metered as "active compute hours") |
| 4a | Claude Code statusline JSON `rate_limits.five_hour.used_percentage`, `seven_day.used_percentage`, `resets_at` (epoch s) | Official percentages identical to `/usage` | High and officially documented — only while a session runs, Pro/Max, after first API response | Passive; requires a statusline hook in `settings.json` |
| 4b | claude.ai → Settings → Usage; `/usage` (alias `/stats`) | Session %, weekly %, per-model | Official, UI only | — |
| 4c | Admin API `usage_report` | Org-level | Not applicable | — |
| 5 | Claude Desktop local files | No readable usage file found (UNVERIFIED negative) | — | — |
| 6 | `claude.ai/api/organizations/{org_id}/usage` with `sessionKey` cookie | Same as settings page | Medium — cookie extraction | Undocumented; ToS disclaimers; cookie rotation |

#### B.1.1 OAuth usage endpoint details
- Headers: `Authorization: Bearer <accessToken>`, `anthropic-beta: oauth-2025-04-20`, `Content-Type: application/json`, and critically `User-Agent: claude-code/<version>` ("without it you hit an aggressively rate-limited bucket and get persistent 429s"); safe polling ≈ 180 s; rate limiting per access token. https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor/issues/202 , https://lib.rs/crates/claude-usage
- Response example with `seven_day_sonnet`, `seven_day_opus: null`, `extra_usage`: https://pypi.org/project/ccusage/
- Scope: token needs `user:profile`; `claude setup-token` tokens lack it: https://gist.github.com/jtbr/4f99671d1cee06b44106456958caba8b
- Breakage: anthropics/claude-code #31021 (2026-03-05) persistent 429 even for `/usage`; closed not planned. https://github.com/anthropics/claude-code/issues/31021
- Reset semantics: 12-day log (June 2026) saw the weekly counter drop every ~72 h while `resets_at` pointed to a 7-day boundary — single account, UNVERIFIED generalization. https://gist.github.com/monperrus/3ac4b303a84946bbeaf2b1123ee99491
- Not documented on code.claude.com or platform.claude.com.

#### B.1.2 Policy risk
- 2026-02-19/20 legal-and-compliance page: "Using OAuth tokens obtained through Claude Free, Pro, or Max accounts in any other product, tool, or service — including the Agent SDK — is not permitted and constitutes a violation of the Consumer Terms of Service." No distinction between inference and read-only. https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access/ , https://gigazine.net/gsc_news/en/20260220-anthropic-third-party-block/
- 2026-01-09 server-side blocks hit inference harnesses (OpenClaw, OpenCode, Roo Code, Goose); the usage endpoint was not blocked — tools shipped through 2026-09-01. Tolerated in practice, prohibited on paper. https://abit.ee/en/artificial-intelligence/anthropic-claude-code-oauth-openclaw-opencode-claude-max-subscription-api-ban-terms-of-service-en
- Policy-clean tools: (a) JSONL only (aqua5230/usage), (b) statusline JSON (bartosz-warzocha, Claude-Code-Usage-Monitor v4), (c) drive the real `claude` binary via PTY and scrape `/usage` (CodexBar fallback, CodexBar-Win, ClaudeBar).

#### B.2 Credential storage (official docs fetched 2026-09-03: https://code.claude.com/docs/en/authentication)
- macOS: Keychain item service `Claude Code-credentials`, account `$USER` (`security find-generic-password -s "Claude Code-credentials" -a "$USER" -w`); falls back to `~/.claude/.credentials.json` (0600) when Keychain write fails. Claude Code deletes the JSON and migrates into Keychain on mac. https://gist.github.com/Prajwalsrinvas/cacbb728c4ea06c3bc1676608d3c72dc
- Windows: `%USERPROFILE%\.claude\.credentials.json`, ACL-protected only. No Credential Manager for OAuth tokens (issue #29049; plugin credentials go to the keychain since v2.1.83, not the login token). https://github.com/anthropics/claude-code/issues/29049 , https://dev.to/rsdouglas/claude-code-plugin-credentials-what-the-new-keychain-storage-does-and-doesnt-do-cnf
- Linux: `~/.claude/.credentials.json` 0600.
- `CLAUDE_CONFIG_DIR` relocates the file and keys the Keychain entry — tools must honour it.
- JSON shape: `{"claudeAiOauth": {"accessToken": "sk-ant-oat01-…", "refreshToken": "sk-ant-ort01-…", "expiresAt": <ms epoch>, "scopes": ["user:inference","user:profile"]}}`; `subscriptionType` UNVERIFIED.
- Refresh: ~60 min expiry; Claude Code refreshes on its own runs. Third-party monitors spawn `claude update` / `claude -p` to force refresh (jens-duttke) or hit the token endpoint with Claude Code's client id (host/path reports conflict — UNVERIFIED). Refreshing yourself is the clearest "third-party use" and can race Claude Code's rotation. https://github.com/anthropics/claude-code/issues/50743 , https://gist.github.com/shubcodes/3c9c7ff813715aa47018bf22e7cf8cb5
- Keychain ACL weakness (Silverfort 2026-07-24, Claude Code 2.1.185): item created without ACL, readable by any user process; Anthropic "tracking a tightening". https://www.silverfort.com/blog/skipping-the-lock-a-claude-code-cli-weakness-lets-any-macos-process-read-stored-credentials/
- `~/.config/anthropic` / `%APPDATA%\Anthropic`: Anthropic *profile* directory for the `ant` CLI / WIF / Console sign-in — Console/API OAuth profiles, not subscription tokens. A Max user was billed API credits because of a leftover profile (issue #84394). https://github.com/anthropics/claude-code/issues/84394

#### B.3 Local transcript parsing (ccusage)
- Paths: `~/.claude/projects/**/*.jsonl`, `~/.config/claude/projects/**`, `CLAUDE_CONFIG_DIR` override. https://ccusage.com/guide/
- Fields: `timestamp`, `message.model`, `message.id`, `requestId`, `message.usage.{input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens}`, legacy `costUSD`. Dedup by `message.id` + `requestId` (PARTIALLY VERIFIED — in source, not the guide page). Pricing: LiteLLM `model_prices_and_context_window.json`.
- Limitations: Claude Code only; no account limit known, so any "% of window" from JSONL is an estimate.

#### B.4 Official surfaces
- Statusline JSON (documented): https://code.claude.com/docs/en/statusline — `rate_limits.five_hour.used_percentage`, `rate_limits.seven_day.used_percentage` (0–100), `rate_limits.*.resets_at` (Unix epoch seconds). "`rate_limits` appears only for Claude.ai Pro and Max subscribers … and only after the first API response in the session. Each window may be independently absent, and Claude Code drops a window once its `resets_at` time passes." Claude Code re-runs the statusline script when a window reaches `resets_at`.
- `/usage` (alias `/stats`) shows plan usage bars (ClaudeLog); support article 14552983 documents only `/cost` and `/context`. https://claudelog.com/faqs/what-is-stats-command-in-claude-code/ , https://support.claude.com/en/articles/14552983-models-usage-and-limits-in-claude-code
- Support 11647753: all product surfaces count toward the same limit. https://support.claude.com/en/articles/11647753-how-do-usage-and-length-limits-work
- No subscriber-facing usage API in Anthropic docs.

#### B.5 Claude Desktop
No locally readable usage file found (UNVERIFIED negative). https://github.com/anthropics/claude-code/issues/41490

### C. Framework options

Signing/cost facts (2026):
- Apple Developer Program $99/yr; free accounts cannot notarize. https://developer.apple.com/programs/ , https://v2.tauri.app/distribute/sign/macos/
- macOS Sequoia+ removed the Control-click bypass: System Settings → Privacy & Security → "Open Anyway" + admin password, or `xattr -d -r com.apple.quarantine`. Unsigned Tauri builds on Apple Silicon show "app is damaged"; ad-hoc signing (`"signingIdentity": "-"`) avoids that but not the Gatekeeper prompt. https://www.idownloadblog.com/2024/08/07/apple-macos-sequoia-gatekeeper-change-install-unsigned-apps-mac/
- Windows (Microsoft doc 2026-05-04): unsigned → "Windows protected your PC" → "Run anyway"; "EV certificates no longer bypass SmartScreen"; reputation takes weeks; Win11 Smart App Control blocks unsigned outright; enterprise policy can forbid "Run anyway". https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation
- Azure Artifact Signing from $9.99/mo; Public-Trust identity for individuals US/CA only; organizations incl. KR (conflicting reports, UNVERIFIED). https://learn.microsoft.com/en-nz/answers/questions/5810735/cant-create-a-new-trusted-signing-individual-ident , https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart
- OV code-signing certs ~$64–$226/yr. https://comparecheapssl.com/windows-code-signing-certificate/
- SignPath Foundation: free OV signing for public OSS. https://signpath.org/terms.html

| Framework | Bundle | Tray / always-on-top / transparent | Effort | Install w/o dev tools | Unsigned experience | Notes |
|---|---|---|---|---|---|---|
| Tauri v2 | ~3 MB hello-world; 6–8 MB real tray monitors (xinggaoya/system-monitor) | `tray-icon` feature Win+mac; `alwaysOnTop`, `decorations:false`, `transparent`, `skipTaskbar`; mac transparency needs `macOSPrivateApi: true`; Win transparent-window bug (issue #13415) | Medium (Rust for builder only) | .msi/.exe (NSIS/WiX), .dmg; WebView2 bootstrapper | Same as native binary; updater plugin needs minisign | Precedents: Win-CodexBar, xinggaoya/system-monitor (Tauri 2.2 + sysinfo + nvml-wrapper), NeoHtop. https://v2.tauri.app/learn/system-tray/ , https://v2.tauri.app/plugin/updater/ , https://dev.to/hiyoyok/complete-guide-to-building-a-macos-menu-bar-app-with-tauri-v2-aji , https://github.com/tauri-apps/tauri/issues/13415 |
| Electron | 85–200 MB installers; 150–200 MB RSS idle | Full | Low–medium | electron-builder NSIS/DMG | Same | Heaviest for a tiny widget. https://www.pkgpulse.com/guides/electron-vs-tauri-2026 |
| Python (pystray/pywebview + PyInstaller) | ~12.5 MB single EXE (jens-duttke); typically 20–40 MB | pystray, tkinter `-topmost`, pywebview | Low | One-file EXE / .app | PyInstaller .app often flagged by AV/Gatekeeper | https://www.intego.com/mac-security-blog/pyinstaller-macos-malware-vector/ |
| Avalonia (.NET) | ~58 MB self-contained; ~18 MB NativeAOT+UPX | `TrayIcon`, `Topmost`, `TransparencyLevelHint` | Medium | dotnet publish | Same | https://avaloniaui.net/maui-compare |
| .NET MAUI | Larger | Tray/multi-window harder; no Linux | Medium–high | MSIX/.app | Same | Not recommended |
| Flutter desktop | Tens of MB | `tray_manager` + `window_manager` | Medium | .exe/.app | Same | No first-party system-info plugin |
| Swift + C# dual native | Smallest | Best | High (two codebases) | .dmg + brew; .exe/winget | Same | What the market did (CodexBar + ports) |
| Web-only page | 0 | No tray/always-on-top/local access; CORS | Trivial | URL | n/a | Not viable alone |

### D. Sampling CPU / memory / GPU

D.1 CPU/memory: Rust `sysinfo` 0.39.6 (`refresh_cpu_usage()` twice ≥ `MINIMUM_CPU_UPDATE_INTERVAL`, `global_cpu_usage()`, `total_memory()/used_memory()`), https://docs.rs/sysinfo/latest/sysinfo/ ; Python `psutil` 7.x https://psutil.io/ ; Node `systeminformation` (spawns PowerShell on Windows) https://github.com/sebhildebrandt/systeminformation ; .NET `PerformanceCounter` (Win) / interop (mac); Swift Mach APIs.

D.2 GPU:
- NVIDIA (Win/Linux): NVML https://developer.nvidia.com/management-library-nvml ; Rust `nvml-wrapper` 0.13.0 (2026-08-31), runtime-loads `nvml.dll`, `utilization_rates()`, `memory_info()` https://github.com/Cldfire/nvml-wrapper ; Python `nvidia-ml-py` (GPUtil dead since 2018) https://pypi.org/project/nvidia-ml-py/ ; Node `systeminformation.graphics()` via nvidia-smi subprocess https://systeminformation.io/graphics.html
- Windows vendor-agnostic: PDH `\GPU Engine(*)\Utilization Percentage`, instances `pid_1234_engtype_3D_0`, enumerate via `PdhEnumObjectItemsW`; Task Manager = max across engines. VRAM `\GPU Adapter Memory(*)\Dedicated Usage` (UNVERIFIED name). https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/ ; Rust `windows` crate `Win32::System::Performance`, `precord` crate implements `system_gpu_usage()` https://crates.io/crates/precord ; oshi PR #2114 reference https://github.com/oshi/oshi/pull/2114
- macOS Apple Silicon (no sudo): (1) IOKit `IOAccelerator`/`AGXAccelerator` `PerformanceStatistics` → `Device Utilization %` — `gpuinfo` (C) https://github.com/andersrennermalm/gpuinfo , `gpuer` https://github.com/simonw/gpuer , telegraf plugin https://github.com/XReyRobert/macos-telegraf-gpumonitoring ; idle-high caveat https://airstats.app/blog/gpu-utilisation-98-percent-idle ; (2) IOReport private framework — `macmon` (Rust, 1.9k stars) https://github.com/vladkens/macmon , `macpow` crate https://crates.io/crates/macpow ; (3) `powermetrics` requires root (asitop) https://github.com/tlkh/asitop
- macOS Intel/AMD: same IOKit route (`IntelAccelerator`, `AMDRadeonX6000_*`); IOReport tools don't support Intel. https://github.com/aristocratos/btop/issues/1763

D.3 Verdict: Tauri/Rust is the only stack with all-in-process sampling on both OSes (sysinfo + nvml-wrapper + PDH + IOKit), proven by xinggaoya/system-monitor at 6–8 MB.

D.4 Reference monitors: Stats (exelban, Swift, 41.6k★) https://github.com/exelban/stats ; macmon https://github.com/vladkens/macmon ; xinggaoya/system-monitor (Tauri 2.2 + Vue, sysinfo + nvml-wrapper) https://github.com/xinggaoya/system-monitor ; NeoHtop https://abdenasser.github.io/neohtop/ ; btop https://github.com/aristocratos/btop

### E. What to copy
1. Layered data-source chain with explicit fallback order, each layer labelled in the UI (CodexBar, CodexBar-Win).
2. Ship a statusline hook and read its JSON — the only documented programmatic source, policy-clean.
3. If the OAuth endpoint is used: `User-Agent: claude-code/<version>`, `anthropic-beta: oauth-2025-04-20`, poll ≥180 s, back off on 429, never store the token, honour `CLAUDE_CONFIG_DIR`, let Claude Code refresh.
4. Reset countdowns and per-window bars in the tray icon; track the last observed drop (72-h discrepancy).
5. Local-first, no telemetry, visible "unofficial, may break, may violate ToS" disclaimer.
6. Distribution: brew cask + winget + Releases with SHA-256; SignPath Foundation for Windows if public OSS; $99/yr Apple Developer ID.
7. Keep idle CPU tiny: reuse handles, no per-tick subprocesses, 1–2 s sampling.
8. One codebase, two platforms: Tauri v2.
