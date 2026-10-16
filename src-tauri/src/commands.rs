use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::hotkey::registry::HotkeyFailure;
use crate::settings::validate::{ValidationError, validate};

use crate::settings::model::Settings;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
pub fn export_settings(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let settings = state.settings.read().unwrap().clone();
    state.store.export_to(&settings, Path::new(&path)).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn import_settings(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<Settings, String> {
    let imported = state.store.import_from(Path::new(&path)).map_err(|e| format!("{e:#}"))?;
    *state.settings.write().unwrap() = imported.clone();
    crate::apply_settings(&app);
    Ok(imported)
}

#[tauri::command]
pub fn list_installed_apps() -> Vec<crate::launcher::InstalledApp> {
    crate::launcher::scan_start_menu()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub hotkey_failures: Vec<HotkeyFailure>,
}

// 동기 커맨드로 유지한다: 메인 스레드에서 실행되어야 apply_settings 의 단축키 등록과 교착하지 않는다.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SaveResult, Vec<ValidationError>> {
    let errors = validate(&settings);
    if !errors.is_empty() {
        return Err(errors);
    }
    state
        .store
        .save(&settings)
        .map_err(|e| vec![ValidationError { field: String::new(), message: format!("{e:#}") }])?;
    *state.settings.write().unwrap() = settings;
    crate::apply_settings(&app);
    Ok(SaveResult { hotkey_failures: state.hotkeys.lock().unwrap().failures() })
}

/// 설정 창에서 단축키를 녹화하는 동안 전역 단축키가 가로채지 않도록 모두 해제/재등록한다.
#[tauri::command]
pub fn set_hotkeys_paused(app: AppHandle, state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    state.hotkeys.lock().unwrap().set_paused(paused);
    crate::apply_settings(&app);
    Ok(())
}

#[tauri::command]
pub fn get_hotkey_failures(state: State<'_, AppState>) -> Vec<HotkeyFailure> {
    state.hotkeys.lock().unwrap().failures()
}
