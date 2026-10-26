use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const MENU_SETTINGS: &str = "settings";
const MENU_QUIT: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "설정 열기", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("Convenient Use")
        .menu(&menu)
        // 왼쪽 클릭은 메뉴 대신 설정 열기에 쓴다 (메뉴는 오른쪽 클릭)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_settings(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_SETTINGS => show_settings(app),
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}

pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn notify_hotkey_failures(app: &AppHandle, count: usize) {
    if count == 0 {
        return;
    }
    notify(app, &format!("단축키 {count}개를 등록하지 못했습니다"));
}

pub fn notify(app: &AppHandle, body: &str) {
    let result = app.notification().builder().title("Convenient Use").body(body).show();
    if let Err(e) = result {
        log::warn!("알림 표시 실패: {e}");
    }
}
