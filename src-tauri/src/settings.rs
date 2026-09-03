//! 사용자 설정(레이아웃·테마·펄스·예시 값·창 위치)과 레이아웃별 창 크기.
//! 저장 위치: OS 앱 설정 폴더의 settings.json (Tauri `app_config_dir`).

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, Runtime};

/// 창 안쪽 여백(px). 등록 마크가 -6px로 튀어나오므로 8px 확보 (디자인 README).
pub const PAD: f64 = 8.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub layout: String, // "2a".."2g"
    pub theme: String,  // "dark" | "light" | "system"
    pub alarm: String,  // "pulse" | "off"
    pub demo: bool,     // 예시 값 표시 (검증용)
    pub scale: f64,     // 화면 배율 1.0 / 1.25 / 1.5 / 1.75
    pub oauth: bool,    // 모델별 한도 조회 (비공식 경로, 기본 꺼짐 — DECISIONS D9)
    pub pos: Option<(i32, i32)>, // 물리 픽셀 좌표
}

impl Default for Settings {
    fn default() -> Self {
        Self { layout: "2a".into(), theme: "system".into(), alarm: "pulse".into(), demo: false,
               scale: 1.25, oauth: false, pos: None }
    }
}

pub struct Store(pub Mutex<Settings>);

/// (id, 메뉴 이름, 판 너비, 판 높이) — 디자인 README "가로형 7종" 표.
pub const LAYOUTS: [(&str, &str, f64, f64); 7] = [
    ("2a", "3칸 분할", 532.0, 132.0),
    ("2b", "계기 3연", 596.0, 136.0),
    ("2c", "초슬림 리본", 560.0, 62.0),
    ("2d", "계기 + 긴 히스토리", 604.0, 152.0),
    ("2e", "눈금 막대 계기판", 508.0, 144.0),
    ("2f", "도트 매트릭스", 520.0, 126.0),
    ("2g", "계기 + 시스템 링 3연", 556.0, 134.0),
];

/// 창 크기 = 레이아웃 확정 크기 × 배율 + 등록 마크 여백. 배율은 창 안의 글씨까지 함께 키운다.
pub fn window_size(layout: &str, scale: f64) -> (f64, f64) {
    let (_, _, w, h) = LAYOUTS.iter().find(|l| l.0 == layout).copied().unwrap_or(LAYOUTS[0]);
    ((w + 2.0 * PAD) * scale, (h + 2.0 * PAD) * scale)
}

fn path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// 설정을 디스크에 쓴다. 실패해도 앱은 계속 돈다 (다음 실행에서 기본값으로 시작).
pub fn save_any<R: Runtime>(app: &AppHandle<R>, s: &Settings) {
    if let Some(p) = path(app) {
        let _ = p.parent().map(fs::create_dir_all);
        let _ = serde_json::to_string_pretty(s).map(|j| fs::write(p, j));
    }
}
