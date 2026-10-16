// 후속 단위에서 사용할 타입이 먼저 정의되어 있다
#![allow(dead_code)]

mod commands;
mod launcher;
mod platform;
mod remap;
mod settings;
mod state;
mod tray;
mod window_layout;

use std::sync::RwLock;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::export_settings,
            commands::import_settings,
            commands::list_installed_apps
        ])
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_settings(app);
        }))
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let store = settings::store::SettingsStore::new(path);
            app.manage(state::AppState { settings: RwLock::new(store.load()), store });
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 상주 앱이므로 창 닫기는 종료가 아니라 숨김으로 처리한다
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
