# Claude Usage — 구독 한도 + CPU/메모리/GPU 상시 표시 창

Windows와 macOS에서 **내 Claude 구독 한도(5시간 창·주간)** 와 **이 PC의 CPU·메모리·GPU 사용률**을
항상-위(always-on-top) 작은 창 하나에 보여 주는 프로그램입니다. 서버가 없고, 각자 자기 계정으로 씁니다.

> 상태 (2026-09-03): **M0 관통 골격** — 항상-위 창 + 트레이 + CPU/메모리 표시, Windows 설치 파일.
> 한도 표시(M1), GPU(M2), macOS 빌드(M3)는 아직입니다. 설계는 [[docs/ARCHITECTURE]] 참조.

## 한도 값을 읽는 방식 (중요)

- **기본**: Claude Code(CLI)가 세션 중 statusline 스크립트에 넘겨 주는 공식 JSON(`rate_limits`)을 읽습니다.
  네트워크 호출이 없고 Anthropic 문서에 있는 경로입니다. Claude Code를 닫아 둔 동안은 마지막 값에 멈춥니다.
- **옵트인(기본 꺼짐)**: Claude Code가 저장한 로그인 토큰을 읽어 비공식 조회 주소를 호출합니다. Anthropic 약관은
  구독 토큰의 제3자 사용을 금지하고 있으며, 조회 용도는 묵인되고 있으나 보장은 없습니다. 켤 때 고지에 동의해야 합니다.
- 두 경우 모두 토큰은 메모리에만 있고 어디에도 저장·전송되지 않습니다. 이 앱은 Anthropic 외에 접속하지 않습니다.

## 빌드 (개발자)

필요: Node 20+, Rust stable, Windows는 MSVC Build Tools(C++ 워크로드), macOS는 Xcode CLT.

```bash
npm install
npm run dev      # 개발 실행
npm run build    # 설치 파일: src-tauri/target/release/bundle/
```

## 설치 (동료)

Releases에서 설치 파일을 받습니다. 서명이 없어서 경고가 뜹니다.
- Windows: "Windows의 PC 보호" → **추가 정보 → 실행**
- macOS: 시스템 설정 → 개인정보 보호 및 보안 → **그래도 열기** (Sequoia부터 "우클릭 → 열기"는 통하지 않습니다)

## 문서

| 문서 | 내용 |
|---|---|
| [[docs/ARCHITECTURE]] | 설계 단일 원천 |
| [[docs/PRIOR-ART-SURVEY]] | 기존 도구·데이터 소스·프레임워크 조사 |
| [[DECISIONS]] | 확정 결정 D1~D9 |
| [[docs/WORKLOG]] | 진행 로그 |
