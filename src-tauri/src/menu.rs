//! 네이티브 메뉴 하나를 창 우클릭과 트레이 양쪽에 쓴다 (디자인 README "인터랙션": 우클릭 → 테마·펄스·설정·종료).
//! 메뉴 id 규칙: "layout:2a" / "theme:dark" / "alarm:pulse" / "demo" / "show" / "hide" / "quit".

use crate::settings::{self, Settings, Store, LAYOUTS};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, LogicalSize, Manager, Runtime,
};

pub fn build<R: Runtime>(app: &AppHandle<R>, s: &Settings, for_tray: bool) -> tauri::Result<Menu<R>> {
    let check = |id: String, text: &str, on: bool| CheckMenuItem::with_id(app, id, text, true, on, None::<&str>);

    let mut layout_items = Vec::new();
    for (id, name, _, _) in LAYOUTS {
        layout_items.push(check(format!("layout:{id}"), &format!("{id} · {name}"), s.layout == id)?);
    }
    let layout = Submenu::with_id(app, "layout", "레이아웃", true)?;
    for it in &layout_items { layout.append(it)?; }

    let scale = Submenu::with_id(app, "scale", "크기", true)?;
    for (v, name) in [(1.0, "100%"), (1.25, "125%"), (1.5, "150%"), (1.75, "175%")] {
        scale.append(&check(format!("scale:{v}"), name, (s.scale - v).abs() < 0.01)?)?;
    }

    let theme = Submenu::with_id(app, "theme", "테마", true)?;
    for (id, name) in [("dark", "다크"), ("light", "라이트"), ("system", "시스템 따라감")] {
        theme.append(&check(format!("theme:{id}"), name, s.theme == id)?)?;
    }

    // Claude 한도 연결 — 사용자의 settings.json을 고치는 동작이므로 명시적으로 고르게 한다.
    let hook = check("hook".into(), "Claude 한도 연결 (statusline 훅)", crate::usage::is_installed())?;
    // 모델별 창(Fable·Opus 등)은 이 경로에만 있다. 약관상 권장되지 않는 비공식 경로다 (DECISIONS D9).
    let oauth = check("oauth".into(), "모델별 한도 표시 (비공식 조회 · 약관 주의)", s.oauth)?;
    let pulse = check("alarm:pulse".into(), "위험(90%↑) 시 숫자 깜빡임", s.alarm == "pulse")?;
    let demo = check("demo".into(), "예시 값 표시 (검증용)", s.demo)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let toggle = if for_tray {
        MenuItem::with_id(app, "show", "창 보이기", true, None::<&str>)?
    } else {
        MenuItem::with_id(app, "hide", "창 숨기기 (트레이로)", true, None::<&str>)?
    };
    let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;

    Menu::with_items(app, &[&layout, &scale, &theme, &sep, &hook, &oauth, &pulse, &demo, &sep, &toggle, &sep, &quit])
}

/// 메뉴 선택 처리: 설정 갱신 → 저장 → 창 크기 → 창에 통보 → 트레이 메뉴 체크 상태 갱신.
pub fn handle<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    match id.as_str() {
        "quit" => { app.exit(0); return; }
        "show" => { show_main(app); return; }
        "hide" => { if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); } return; }
        "hook" => {
            let r = if crate::usage::is_installed() { crate::usage::uninstall() } else { crate::usage::install() };
            if let Err(e) = r {
                let _ = app.emit("usage:error", e);
            }
            // 메뉴의 체크 상태를 새 상태로 다시 그린다.
            let s = app.state::<Store>().0.lock().unwrap().clone();
            if let Some(tray) = app.tray_by_id("main") {
                if let Ok(m) = build(app, &s, true) { let _ = tray.set_menu(Some(m)); }
            }
            return;
        }
        _ => {}
    }
    let store = app.state::<Store>();
    let snapshot = {
        let mut s = store.0.lock().unwrap();
        if let Some(v) = id.strip_prefix("layout:") { s.layout = v.into(); }
        else if let Some(v) = id.strip_prefix("scale:") { s.scale = v.parse().unwrap_or(1.25); }
        else if id == "oauth" { s.oauth = !s.oauth; }
        else if let Some(v) = id.strip_prefix("theme:") { s.theme = v.into(); }
        else if id == "alarm:pulse" { s.alarm = if s.alarm == "pulse" { "off".into() } else { "pulse".into() }; }
        else if id == "demo" { s.demo = !s.demo; }
        else { return; }
        s.clone()
    };
    save_and_apply(app, &snapshot);
}

pub fn save_and_apply<R: Runtime>(app: &AppHandle<R>, s: &Settings) {
    settings::save_any(app, s);
    if let Some(w) = app.get_webview_window("main") {
        let (width, height) = settings::window_size(&s.layout, s.scale);
        let _ = w.set_size(LogicalSize::new(width, height));
    }
    let _ = app.emit("ui:settings", s);
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(m) = build(app, s, true) { let _ = tray.set_menu(Some(m)); }
    }
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
