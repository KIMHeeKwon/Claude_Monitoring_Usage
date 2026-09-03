mod gpu;
mod menu;
mod oauth;
mod settings;
mod sysmon;
mod usage;

use settings::{Settings, Store};
use std::sync::Mutex;
use tauri::{tray::TrayIconBuilder, AppHandle, LogicalSize, Manager, PhysicalPosition, State};

#[tauri::command]
fn get_settings(store: State<Store>) -> Settings {
    store.0.lock().unwrap().clone()
}

#[tauri::command]
fn show_menu(app: AppHandle, store: State<Store>) -> tauri::Result<()> {
    let s = store.0.lock().unwrap().clone();
    let m = menu::build(&app, &s, false)?;
    if let Some(w) = app.get_webview_window("main") {
        w.popup_menu(&m)?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_settings, show_menu])
        .setup(|app| {
            let s = settings::load(app.handle());
            app.manage(Store(Mutex::new(s.clone())));

            // 창: 저장된 레이아웃 크기·위치로 맞춘 뒤 표시 (tauri.conf.json의 visible=false).
            if let Some(w) = app.get_webview_window("main") {
                let (width, height) = settings::window_size(&s.layout, s.scale);
                let _ = w.set_size(LogicalSize::new(width, height));
                if let Some((x, y)) = s.pos { let _ = w.set_position(PhysicalPosition::new(x, y)); }
                let _ = w.show();
            }

            // 트레이: 좌클릭 = 창 보이기, 우클릭 = 같은 설정 메뉴.
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Claude Usage")
                .menu(&menu::build(app.handle(), &s, true)?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| menu::handle(app, e))
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        menu::show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            sysmon::spawn(app.handle().clone());
            usage::spawn(app.handle().clone());
            Ok(())
        })
        // 우클릭 팝업 메뉴의 선택은 앱 수준 핸들러로 온다.
        .on_menu_event(|app, e| menu::handle(app, e))
        .on_window_event(|window, event| match event {
            // 닫기(X)는 숨김. 종료는 트레이/메뉴의 "종료"로만.
            tauri::WindowEvent::CloseRequested { api, .. } => { let _ = window.hide(); api.prevent_close(); }
            // 창 위치 저장 (다음 실행 때 복원).
            tauri::WindowEvent::Moved(p) => {
                let app = window.app_handle();
                let store = app.state::<Store>();
                let snap = { let mut s = store.0.lock().unwrap(); s.pos = Some((p.x, p.y)); s.clone() };
                settings::save_any(app, &snap);
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
