mod clipboard;
mod commands;
mod hotkey;
mod keyboard;
mod launcher;
mod platform;
mod remap;
mod settings;
mod snippet;
mod state;
mod tray;
mod window_layout;

use std::sync::{Mutex, RwLock};

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::keyboard::KeyHandler;
use crate::settings::model::Settings;

pub fn run() {
    tauri::Builder::default()
        // single-instance 는 다른 플러그인보다 먼저 등록해야 두 번째 실행을 바로 넘겨받는다
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_settings(app);
        }))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::export_settings,
            commands::import_settings,
            commands::list_installed_apps,
            commands::save_settings,
            commands::set_hotkeys_paused,
            commands::get_hotkey_failures,
            commands::list_clips,
            commands::get_clip_detail,
            commands::set_pinned,
            commands::delete_clip,
            commands::clear_clips,
            commands::paste_clip,
            commands::hide_clipboard_popup
        ])
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // 키를 뗄 때 이벤트는 무시한다 (누름 한 번에 한 번만 동작)
                    if event.state() == ShortcutState::Pressed {
                        hotkey::registry::dispatch(app, shortcut.id());
                    }
                })
                .build(),
        )
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let store = settings::store::SettingsStore::new(path);
            let data_dir = app.path().app_data_dir()?;
            let clips = clipboard::store::ClipStore::open(&data_dir.join("clipboard.db"), &data_dir.join("clips"))?;
            app.manage(state::AppState {
                settings: RwLock::new(store.load()),
                store,
                hotkeys: Mutex::new(Default::default()),
                clips: Mutex::new(clips),
                clip_suppress_until: Mutex::new(None),
                popup_prev: Mutex::new(None),
            });
            tray::create(app.handle())?;
            clipboard::watcher::start(app.handle().clone());
            keyboard::start();
            let failed = apply_settings(app.handle());
            tray::notify_hotkey_failures(app.handle(), failed);
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                // 상주 앱이므로 창 닫기는 종료가 아니라 숨김으로 처리한다
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                // 클립보드 팝업은 바깥을 클릭하면(포커스 상실) 닫힌다
                tauri::WindowEvent::Focused(false) if window.label() == "clipboard-popup" => {
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 저장·가져오기·시작 시 설정을 실제 시스템 상태(단축키, 자동 시작)에 반영한다. 등록 실패 개수를 돌려준다.
/// 메인 스레드에서 호출해야 한다: 단축키 등록이 메인 스레드를 거치므로, 다른 스레드에서 hotkeys 락을
/// 잡은 채 기다리면 단축키 핸들러(메인 스레드)와 교착할 수 있다.
pub fn apply_settings(app: &AppHandle) -> usize {
    let state = app.state::<state::AppState>();
    let settings = state.settings.read().unwrap().clone();

    // max_items 가 줄어든 경우를 반영한다
    if let Err(e) = state.clips.lock().unwrap().prune(settings.clipboard.max_items) {
        log::warn!("클립보드 정리 실패: {e}");
    }

    let mut hotkeys = state.hotkeys.lock().unwrap();
    hotkeys.reload(app, &settings);
    let failed = hotkeys.failures().len();
    drop(hotkeys);

    keyboard::set_handlers(keyboard_handlers(app, &settings));

    let manager = app.autolaunch();
    match manager.is_enabled() {
        Ok(on) if on != settings.general.autostart => {
            let result = if settings.general.autostart { manager.enable() } else { manager.disable() };
            if let Err(e) = result {
                log::warn!("자동 시작 설정 실패: {e}");
            }
        }
        Err(e) => log::warn!("자동 시작 상태 확인 실패: {e}"),
        _ => {}
    }
    failed
}

/// 키보드 훅 핸들러 체인. 순서가 곧 우선순위다 (리매핑 핸들러는 맨 앞에 추가).
fn keyboard_handlers(app: &AppHandle, settings: &Settings) -> Vec<Box<dyn KeyHandler>> {
    let mut handlers: Vec<Box<dyn KeyHandler>> = Vec::new();
    if settings.general.features.remap {
        handlers.push(Box::new(remap::RemapHandler::new(&settings.remap)));
    }
    if settings.general.features.snippet {
        handlers.push(Box::new(snippet::SnippetHandler::new(app.clone(), &settings.snippets)));
    }
    handlers
}
