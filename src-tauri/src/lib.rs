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
            commands::hide_clipboard_popup,
            commands::apply_backdrop
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
            // 트레이 상주 앱이므로 Dock 아이콘·앱 전환기 항목을 만들지 않는다
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let path = app.path().app_config_dir()?.join("settings.json");
            let mut store = settings::store::SettingsStore::new(path);
            let settings = store.load();
            let mut startup_warnings: Vec<String> = store.save_blocked().map(str::to_owned).into_iter().collect();
            let data_dir = app.path().app_data_dir()?;
            let images_dir = data_dir.join("clips");
            // 클립보드 DB 하나 때문에 앱 전체가 뜨지 못하지 않도록, 열 수 없으면 이번 실행만 메모리 DB 로 대신한다
            let clips = match clipboard::store::ClipStore::open(&data_dir.join("clipboard.db"), &images_dir) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("클립보드 DB 를 열지 못했습니다: {e}");
                    startup_warnings.push("클립보드 기록을 열지 못해 이번 실행 동안만 임시로 기록합니다".into());
                    clipboard::store::ClipStore::open_in_memory(&images_dir)?
                }
            };
            app.manage(state::AppState {
                settings: RwLock::new(settings),
                store,
                hotkeys: Mutex::new(Default::default()),
                clips: Mutex::new(clips),
                clip_suppress_until: Mutex::new(None),
                popup_prev: Mutex::new(None),
            });
            // 창은 config 에서 create: false 로 두고 상태 등록 뒤에 만든다. Tauri 는 config 창을 setup 보다
            // 먼저 띄우므로, 콜드 부팅처럼 느린 환경에서는 페이지의 get_settings 가 상태 등록 전에 실행되어 실패한다
            for config in &app.config().app.windows {
                tauri::WebviewWindowBuilder::from_config(app.handle(), config)?.build()?;
            }
            tray::create(app.handle())?;
            clipboard::watcher::start(app.handle().clone());
            keyboard::start();
            let failed = apply_settings(app.handle());
            tray::notify_hotkey_failures(app.handle(), failed);
            for warning in &startup_warnings {
                tray::notify(app.handle(), warning);
            }
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
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            // macOS 의 CapsLock 가로채기(hidutil)는 프로세스가 끝나도 남으므로 종료 시 되돌린다
            if let tauri::RunEvent::Exit = event {
                platform::current::hook::capture_caps_lock(false);
            }
        });
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
    platform::current::hook::capture_caps_lock(
        settings.general.features.remap && remap::uses_input_key(&settings.remap, "CapsLock"),
    );

    sync_autostart(app, settings.general.autostart);
    failed
}

/// 개발 빌드는 자동 시작 등록을 건드리지 않는다. 설정 파일을 설치본과 공유하므로 여기서 등록하면
/// 시작 프로그램이 개발 빌드 exe 를 가리키게 된다.
fn sync_autostart(app: &AppHandle, enabled: bool) {
    if std::env::current_exe().is_ok_and(|exe| is_cargo_build(&exe)) {
        return;
    }
    let manager = app.autolaunch();
    let result = if enabled {
        // is_enabled 는 등록 항목 이름만 보고 경로는 확인하지 않으므로, 켜져 있어도 현재 exe 경로로 다시 쓴다
        manager.enable()
    } else {
        match manager.is_enabled() {
            Ok(true) => manager.disable(),
            other => other.map(|_| ()),
        }
    };
    if let Err(e) = result {
        log::warn!("자동 시작 설정 실패: {e}");
    }
}

/// cargo 출력 폴더(`target/debug`, `target/<triple>/release` 등) 안의 exe 인지
fn is_cargo_build(exe: &std::path::Path) -> bool {
    let profile_dir = exe.parent().and_then(|p| p.file_name());
    matches!(profile_dir.and_then(|n| n.to_str()), Some("debug" | "release"))
        && exe.ancestors().any(|a| a.file_name().is_some_and(|n| n == "target"))
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

#[cfg(test)]
mod tests {
    use super::is_cargo_build;
    use std::path::Path;

    #[test]
    fn cargo_build_is_detected_by_output_dir() {
        assert!(is_cargo_build(Path::new("/src/app/src-tauri/target/release/app.exe")));
        assert!(is_cargo_build(Path::new("/src/app/target/x86_64-apple-darwin/debug/app")));
        assert!(!is_cargo_build(Path::new("/Users/u/AppData/Local/Convenient Use/convenient-use.exe")));
        assert!(!is_cargo_build(Path::new("/Applications/Convenient Use.app/Contents/MacOS/convenient-use")));
    }
}

