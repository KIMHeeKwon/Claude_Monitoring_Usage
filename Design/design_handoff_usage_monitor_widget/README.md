# Handoff: Claude 사용량 + 시스템 모니터 상시 표시 위젯

## 개요

Windows·macOS에서 **자기 Claude 구독 한도(5시간 창·주간·Opus)** 와 **이 PC의 CPU·메모리·GPU**를
하나의 작은 항상-위(always-on-top) 창에 보여 주는 상주 프로그램의 화면 시안이다.
서버 없음, 각자 자기 계정. 설계 본문은 소스 저장소의 `docs/ARCHITECTURE.md`(v0.1)이고,
이 폴더는 그 문서 §D3이 말한 **화면 시안 확정본**에 해당한다.

채택 범위: **이 폴더의 시안 전부**. 가로형 7종 × 다크/라이트 2테마(14화면)가 구현 대상이고,
턴 1의 7종은 폼팩터 대안 + 상태·크기 규칙의 참조본으로 함께 남긴다.

## 디자인 파일에 대해

`Usage Monitor Widget.dc.html`은 **HTML로 만든 디자인 참조물**이다 — 의도한 외형과 동작을
보여 주는 프로토타입이며, 그대로 복사해 출하할 제품 코드가 아니다.
할 일은 이 HTML이 보여 주는 화면을 **대상 코드베이스의 환경에서 다시 구현**하는 것이다.
이 프로젝트의 환경은 아직 비어 있으므로 `docs/ARCHITECTURE.md` §9의 결론을 따른다:

- **Tauri v2 (Rust 코어 + WebView)** — 상주 메모리·시스템 지표 직접 읽기 때문에 1순위.
  Electron으로 결정되면 모듈 이름은 그대로 두고 언어만 바뀐다.
- 창(WebView) 쪽은 코어가 보내는 **JSON 이벤트 두 종류만** 받아 그린다. 창은 토큰·파일·OS API를
  직접 만지지 않는다. 이 시안의 모든 값은 그 두 이벤트로만 그려진다 (아래 §데이터 계약 매핑).
- 프레임워크는 자유(바닐라 + CSS 변수로 충분한 규모다). 다만 **테마는 CSS 변수 한 세트를
  바꿔치기하는 방식**으로 구현할 것 — 다크/라이트가 레이아웃을 1픽셀도 바꾸지 않는다.

시안은 값이 1초마다 도는 가짜 데이터로 살아 있다. 실물에서는 `sys:update` 1~2초,
`usage:update` 이벤트 수신 시 갱신.

## 충실도

**High-fidelity (hifi).** 색·타이포·간격·크기 전부 확정값이다. 아래 표의 hex·px를 그대로 쓸 것.
글꼴은 Barlow / Barlow Condensed(+ 한글 Noto Sans KR)를 앱에 **번들**한다 (상주 앱이 구글 폰트를
네트워크로 받아오게 두지 않는다).

시각 언어는 **Industry** 디자인 시스템(청강 블루프린트) — 사각 모서리, 헤어라인 테두리,
네 귀 등록 마크. `industry-styles.css`가 토큰 원본이고, 이 폴더의 시안은 그 값들을 인라인으로
적어 둔 것이다. 새 색을 발명하지 말 것.

---

## 디자인 토큰

### 지반·선 (다크 / 라이트 짝)

| 역할 | 다크 (턴 2) | 라이트 (턴 3) |
|---|---|---|
| 창 지반 | `#1d2d3d` | `#f5f5f8` |
| 창 테두리 (1px) | `rgba(148,188,227,.24)` | `rgba(29,31,32,.18)` |
| 구획선 (1px) | `rgba(148,188,227,.18)` | `rgba(29,31,32,.14)` |
| 얕은 구획선 | `rgba(148,188,227,.12)` | `rgba(29,31,32,.10)` |
| 막대·계기 트랙 | `rgba(148,188,227,.16)` | `rgba(29,31,32,.12)` |
| 눈금선 / 그래프 격자 | `rgba(148,188,227,.22)` | `rgba(29,31,32,.18)` |
| 그래프 면적 채움 | `rgba(148,188,227,.18)` | `rgba(89,128,166,.16)` |
| 그래프 2차 면적 (GPU) | `rgba(89,126,163,.35)` | `rgba(65,97,128,.20)` |
| 그래프 바탕 | `rgba(148,188,227,.05~.07)` | `rgba(89,128,166,.06~.07)` |
| 주 선/채움 | `#94bce3` | `#5980a6` |
| 2차 선/채움 (Opus·MEM) | `#749dc4` | `#749dc4` |
| 3차 선/채움 (GPU) | `#597ea3` | `#416180` |
| 주 텍스트·숫자 | `#eef6ff` | `#1d1f20` |
| 보조 텍스트 | `rgba(214,235,255, .4 / .5 / .55 / .7)` | `rgba(29,31,32, .4 / .5 / .55 / .7)` |
| 창 그림자 | `0 12px 32px rgba(43,43,45,.34)` | `0 12px 32px rgba(43,43,45,.20)` |

라이트/다크 전환은 위 표의 한 열을 다른 열로 바꾸는 것 **뿐**이다. 크기·간격·글꼴은 공통.

### 상태 색 (두 테마 공통)

| 상태 | 조건 | 색 | 적용 |
|---|---|---|---|
| 정상 | 5H < 75% | `#5980a6` (라이트) / `#94bce3` (다크 계기 채움) | 숫자·막대·계기 호 |
| 주의 | 5H ≥ 75% | `#a68059` | 숫자 + 막대 + 창 테두리 |
| 위험 | 5H ≥ 90% | `#a65959` | 숫자 + 막대 + 창 테두리, **숫자만 펄스** |
| stale | 갱신 끊김 | `rgba(29,31,32,.38)` / 다크는 `rgba(214,235,255,.38)` | 값 유지 + 회색 + 점선 테두리 |

주의·위험 색은 액센트 `#5980a6`과 같은 OKLCH 명도·채도에서 색상만 돌린 값이다. **다른 경고색을
새로 만들지 말 것.** 주간·Opus 막대도 같은 임계값 함수를 쓴다.

펄스: `@keyframes om-pulse { 0%,100%{opacity:1} 50%{opacity:.28} }`, `1.15s ease-in-out infinite`,
**5H 숫자 글리프에만** 적용. 창 전체를 흔들거나 배경을 깜빡이지 않는다. 위험 상태가 아니면
`animation-play-state: paused`. 사용자 설정에서 끌 수 있어야 한다(`alarm: pulse | off`).

### 타이포

| 용도 | 스펙 |
|---|---|
| 큰 숫자 | `Barlow 700`, `font-variant-numeric: tabular-nums`, `letter-spacing:-.03em`, `line-height:1` — 17 / 26 / 30 / 34 / 40 / 44 / 46px (레이아웃별) |
| 숫자 뒤 `%` 기호 | `Barlow 500`, 큰 숫자의 0.3~0.4배 크기, 보조 텍스트 색 |
| 섹션 라벨 (`5H LIMIT`, `CPU`) | `Barlow Condensed 600`, 8.5~9.5px, `letter-spacing:.12~.16em`, `text-transform:uppercase` |
| 행 값 / 본문 숫자 | `Barlow 600`, 11~13px, tabular-nums |
| 한글 본문 | `Barlow 400` + `Noto Sans KR` 폴백, 9.5~12px, `line-height:1.3~1.5` |

폰트 스택: `"Barlow","Noto Sans KR",system-ui,sans-serif` / 라벨은 `"Barlow Condensed",sans-serif`.

### 등록 마크 (모든 창 공통, 생략 금지)

각 창의 네 귀에 11×11px 십자 마크, 창 밖으로 `-6px` 오프셋:

```css
.corner { position:absolute; width:11px; height:11px;
  background:
    linear-gradient(#7a7a7d,#7a7a7d) center/1px 100% no-repeat,
    linear-gradient(#7a7a7d,#7a7a7d) center/100% 1px no-repeat; }
/* tl:top/left -6px · tr:top -6px/right -6px · bl:bottom/left -6px · br:bottom/right -6px */
```

투명 창에서 마크가 잘리지 않도록 창 여백을 8px 이상 확보할 것(Tauri `transparent: true` + 내부 패딩).

### 그래프 기하 (정확히 이 값)

| 요소 | 값 |
|---|---|
| 표본 | 40개 링 버퍼, 1초 간격 (`sys:update`) |
| 스파크라인 (행) | `viewBox 120×18` 또는 `120×20`, `preserveAspectRatio="none"`, 상하 패딩 1.6 |
| 긴 히스토리 (3d/2d) | `viewBox 240×54`, 표시 높이 82px, 격자 20.5px 간격 |
| 세로 그래프 (턴 1의 1d) | `viewBox 160×42` |
| 선 | `stroke-width:1.4~1.5`, `vector-effect="non-scaling-stroke"`, `fill:none` |
| 면적 | 같은 점열 + `0,H` / `W,H` 두 점을 붙인 polygon |
| 반원 계기 (큰) | `path d="M12 78 A62 62 0 0 1 136 78"`, `stroke-width:9`, **길이 194.7** → `stroke-dasharray: 194.7*p/100 194.7` |
| 반원 계기 (작은, 3연) | `path d="M8 50 A40 40 0 0 1 88 50"`, `stroke-width:8`, **길이 125.7** |
| 링 (한도, 턴 1의 1e) | `r=36`, `stroke-width:8`, 둘레 **226.2**, `transform="rotate(-90 cx cy)"` |
| 링 (시스템 3연, 2g/3g) | `r=22`, `stroke-width:5`, 둘레 **138.2** |
| 세로 막대 눈금 (2e/3e) | `repeating-linear-gradient(to top, TICK 0 1px, transparent 1px 16.5px)` — 5칸, 막대 26×66px |
| 도트 매트릭스 (2f/3f) | `radial-gradient(circle at 5.5px 5.5px, DOT 2.1px, transparent 2.4px) 0 0/11px 11px`, 높이 44px, 채움 레이어를 `width:{5H%}`로 클립 |

**주의:** 계기 호의 `stroke-dasharray` 기준값은 실제 `getTotalLength()`와 반드시 일치해야 한다
(초기 시안에서 163.4로 잘못 잡아 계기가 절대 100%에 닿지 않는 버그가 있었다). 경로를 바꾸면
기준값을 다시 재라. SVG `<text>` 안에 값을 넣지 말고 숫자는 HTML 오버레이로 겹칠 것(같은 이유로
링 중앙 숫자를 HTML로 바꿨다).

---

## 화면 — 가로형 7종 (채택본, 각 다크/라이트 2벌)

공통 규칙:

- 타이틀바 없음. **왼쪽 상단 영역이 드래그 핸들**(`cursor:grab`, Tauri `data-tauri-drag-region`).
- 창 크기 고정(리사이즈 없음). 우클릭 → 설정·테마·종료 메뉴(시안 없음, 네이티브 메뉴로).
- 오른쪽 위 또는 헤더에 **값의 출처 배지** — `STATUSLINE` / `OAUTH`, 앞에 5~6px 점.
  설계 §5.1(계층 소스)대로 어느 경로의 값인지 항상 보인다.
- 시스템 지표는 항상 뜬다. 한도 조회가 실패해도 창은 산다.
- GPU를 못 읽으면 해당 행/링/막대를 **숨긴다**(자리를 비워 두지 않는다).

| id | 이름 | 크기 | 그리드 | 무엇에 폭을 쓰나 |
|---|---|---|---|---|
| 2a / 3a | 3칸 분할 | 532×132 | `168px 150px 1fr` | 계기 · 한도 막대 · 시스템 3행 |
| 2b / 3b | 계기 3연 | 596×136 | 헤더 + `1fr 1fr 1fr 190px` | 5H·주간·Opus를 같은 계기로 비교 |
| 2c / 3c | 초슬림 리본 | 560×62 | `auto 96px 1fr 1fr 1fr` + 하단 3px 진행선 | 최소 면적, 한도는 숫자 + 3px 선 |
| 2d / 3d | 계기 + 긴 히스토리 | 604×152 | `186px 1fr` | 폭 = 시간축, 40초 CPU/GPU/MEM 겹침 |
| 2e / 3e | 눈금 막대 계기판 | 508×144 | 헤더 + `repeat(5,1fr)` | 다섯 값을 같은 0–100 자에 세움 |
| 2f / 3f | 도트 매트릭스 | 520×126 | `1fr 168px` | 한도를 셀 수 있는 칸으로 |
| 2g / 3g | 계기 + 시스템 링 3연 | 556×134 | `196px 1fr` | 계기 언어를 시스템까지 |

각 레이아웃의 내부 패딩·간격은 시안 HTML의 인라인 값을 그대로 읽을 것 (대부분 헤더 6~7px,
본문 10~14px, 행 간격 7~9px, 라벨-값 그리드 `28~30px 1fr 36~40px`).

### 표시 문구 (확정 카피)

- `5H LIMIT` / `WEEK` / `OPUS` / `CPU` / `MEM` / `GPU` / `VRAM` — 라벨은 영문 대문자.
- 한글은 값 설명에만: `주간`, `Opus`, `{n}시간 {m}분 후 초기화`, `{n}시간 {m}분 남음`, `4일 후 초기화`,
  `Claude Code 로그인 필요`, `{n}분 전 값`, `조회 불가 — 앱 업데이트 필요`.
- 시계는 `HH:MM`, 메모리는 `19.6GB`, VRAM은 `2.1GB`, 온도는 `68°C`.

## 화면 — 폼팩터 참조본 (턴 1)

구현 순위는 낮지만 규칙의 원본이다:

- **1a** 가로 계기 띠 400×88 · **1b** 다크 세로 계기판 264×336 · **1c** 스펙 시트 표 372×248 ·
  **1d** 세로 테이프 컬럼 184×392 · **1e** 링 격자 296×300
- **1f 상태 4종** — 정상 / 주의 / 위험(펄스) / stale의 표현 규칙. 위 §상태 색 표의 근거.
- **1g 크기 3단계** — 트레이 배지 26px(숫자 하나) → 캡슐 176×40(숫자 + CPU 스파크라인) →
  전체(400×88). 접힘/펼침을 만든다면 이 세 단계를 그대로 쓴다.

---

## 인터랙션 & 상태

| 이벤트 | 동작 |
|---|---|
| 드래그 (헤더/왼쪽 상단) | 창 이동. 위치는 종료 시 저장·복원 |
| 클릭 (캡슐 모드) | 전체 크기로 펼침 (1g) |
| 우클릭 | 네이티브 메뉴: 테마(다크/라이트/OS 따라감) · 펄스 켜기·끄기 · 크기 단계 · 설정 · 종료 |
| 트레이 아이콘 클릭 | 창 표시/숨김 토글 |
| 5H가 75% / 90%를 처음 넘을 때 | 색 전환. 90%부터 숫자 펄스 시작 (설정으로 끌 수 있음) |
| `status !== 'ok'` | 마지막 정상값 유지 + 회색 + 상태 문구. 값이 사라지지 않는다 |

전환 애니메이션: 색·막대 폭은 `transition: 200ms ease-out`. 그 외 움직임은 넣지 않는다
(상주 앱이라 시선을 끌면 방해가 된다). 펄스만 예외.

`docs/ARCHITECTURE.md` §6의 실패 상태 → 화면 매핑:

| status | 화면 |
|---|---|
| `no_source` | 한도 영역에 `설정에서 연결 방법을 고르세요` (막대·계기는 트랙만) |
| `stale` | 값 유지 + 회색 + 점선 테두리 + `{n}분 전 값` |
| `no_token` | 한도 영역에 `Claude Code 로그인 필요` |
| `auth_expired` | 값 회색 + `로그인 갱신 필요` |
| `rate_limited` | 값 유지 + `5분 뒤 재시도` |
| `unreachable` | 값 유지 + `{n}분 전 값` |
| `shape_changed` | `조회 불가 — 앱 업데이트 필요` |
| GPU 미지원 | GPU 행/링/막대 숨김 |

## 상태 관리

창이 들고 있어야 하는 것 전부:

```ts
usage: { source, status, fetched_at, five_hour:{used_pct,resets_at},
         seven_day:{...}, seven_day_opus:{...}|null }   // usage:update 그대로
sys:   { cpu_pct, mem:{used_gb,total_gb}, gpu:{name,util_pct,mem_used_gb,mem_total_gb}|null }
history: { cpu:number[40], mem:number[40], gpu:number[40] }  // 창에서만 유지하는 링 버퍼
ui:    { theme:'dark'|'light'|'system', layout:'2a'|…|'2g', size:'badge'|'capsule'|'full',
         alarm:'pulse'|'off' }                          // 로컬 설정에 저장
```

파생값(전부 렌더 시 계산, 상태로 두지 말 것): 임계 색, `stroke-dasharray`, 막대 폭 %,
스파크라인 점열, `resets_at` → `{n}시간 {m}분` 문자열.

### 데이터 계약 매핑

| 화면 값 | 이벤트 필드 |
|---|---|
| 5H 숫자·계기·막대 | `usage.five_hour.used_pct` |
| `{n}시간 {m}분 후 초기화` | `usage.five_hour.resets_at` − now |
| 주간 / Opus | `usage.seven_day.used_pct` / `seven_day_opus.used_pct` (null이면 행 숨김) |
| 출처 배지 | `usage.source` |
| 상태 문구·회색 처리 | `usage.status`, `usage.fetched_at` |
| CPU 행 | `sys.cpu_pct` |
| MEM 행 (`19.6GB` / `61%`) | `sys.mem.used_gb`, `total_gb` |
| GPU 행 · VRAM | `sys.gpu.util_pct`, `mem_used_gb` |

시안에 있으나 계약에 **없는** 값: 온도/팬(`68°C`), `RTX 4090` 이름 외 상세, 모델별 분포, 요청 수.
온도·GPU 이름은 `sys:update`를 확장하면 되고, 모델별 분포·요청 수는 소스가 없으므로
구현하지 말 것(§5.2 로컬 기록 집계는 M5).

## 에셋

없음. 아이콘·이미지를 쓰지 않는다 — 모든 그래픽이 CSS·SVG로 그려진다.
아이콘이 필요해지면 Industry 규칙대로 **Lucide, stroke-width 1.5**만 쓴다.
글꼴만 번들: Barlow(400/500/600/700), Barlow Condensed(600), Noto Sans KR(400/500/700).

## 파일

| 파일 | 무엇 |
|---|---|
| `Usage Monitor Widget.dc.html` | 시안 전체(턴 3 라이트 7종 → 턴 2 다크 7종 → 턴 1 폼팩터 7종). 브라우저로 바로 열림. 각 시안의 인라인 스타일이 스펙 원본 |
| `industry-styles.css` | Industry 디자인 시스템 토큰 원본(색 램프·간격·타이포). 새 값이 필요할 때 여기서 고를 것 |
| `industry-readme.md` | Industry 사용 규칙(블루프린트 프레임·등록 마크·금지 사항) |
| `support.js` | 시안 HTML이 브라우저에서 그대로 열리게 하는 런타임. 구현에는 쓰지 않는다 |
| `screenshots/turn3-light-horizontal.png` | 라이트 가로형 7종(3a–3g) — 채택본 |
| `screenshots/turn2-dark-horizontal.png` | 다크 가로형 7종(2a–2g) — 채택본 |
| `screenshots/turn1-formfactors.png` | 폼팩터 참조본 7종(1a–1g, 상태 4종·크기 3단계 포함) |

스크린샷은 2배 해상도 캡처이고, 창 크기·색은 표에 적힌 값이 원본이다(픽셀을 스포이드로 재지 말 것).

`Usage Monitor Widget.dc.html`의 값은 1초마다 도는 **가짜 데이터**다. 상단 좌측 값들이
움직이는 것은 의도된 것이고, 위험 상태를 보려면 파일의 로직 클래스에서 `fiveHourPct`
기본값(74)을 90 이상으로 바꿔 열면 된다.

## 구현 체크리스트

1. 관통 골격: 항상-위 투명 창 + 트레이 + `2a`(또는 선택 레이아웃) 껍데기, 등록 마크가 잘리지 않게 여백 확보
2. 토큰을 CSS 변수 두 세트(다크/라이트)로 심고 `prefers-color-scheme` + 수동 전환
3. `sys:update` → 링 버퍼 40 → 스파크라인/막대/링
4. `usage:update` → 계기 기준값 재확인(`getTotalLength()`) → 임계 색 → 펄스
5. `status` 7종 전부를 §6 표대로 화면에 태움 (정상만 만들고 끝내지 말 것)
6. 나머지 6개 레이아웃을 같은 값·같은 토큰으로 추가, 설정에서 선택
7. 크기 3단계(1g)와 위치 저장
