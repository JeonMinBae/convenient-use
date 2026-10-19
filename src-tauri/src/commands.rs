use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::hotkey::registry::HotkeyFailure;
use crate::settings::validate::{ValidationError, validate};

use crate::settings::model::Settings;
use crate::settings::store::SettingsStore;
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
pub fn import_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Settings, Vec<ValidationError>> {
    let imported = import_validated(&state.store, Path::new(&path))?;
    *state.settings.write().unwrap() = imported.clone();
    crate::apply_settings(&app);
    Ok(imported)
}

/// 저장과 같은 검증을 통과한 경우에만 설정 파일을 교체한다. 실패하면 현재 파일은 그대로다.
fn import_validated(store: &SettingsStore, src: &Path) -> Result<Settings, Vec<ValidationError>> {
    let whole = |e: anyhow::Error| vec![ValidationError { field: String::new(), message: format!("{e:#}") }];
    let imported = store.read_from(src).map_err(whole)?;
    let errors = validate(&imported);
    if !errors.is_empty() {
        return Err(errors);
    }
    store.save(&imported).map_err(whole)?;
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

const CLIP_PAGE_SIZE: u32 = 100;

#[tauri::command]
pub fn list_clips(
    state: State<'_, AppState>,
    query: Option<String>,
    offset: Option<u32>,
) -> Result<Vec<crate::clipboard::store::ClipItem>, String> {
    state.clips.lock().unwrap().list(query.as_deref(), CLIP_PAGE_SIZE, offset.unwrap_or(0)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_clip_detail(state: State<'_, AppState>, id: i64) -> Result<Option<crate::clipboard::store::ClipDetail>, String> {
    state.clips.lock().unwrap().detail(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pinned(state: State<'_, AppState>, id: i64, pinned: bool) -> Result<(), String> {
    state.clips.lock().unwrap().set_pinned(id, pinned).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clip(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.clips.lock().unwrap().delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_clips(state: State<'_, AppState>) -> Result<(), String> {
    state.clips.lock().unwrap().clear_unpinned().map_err(|e| e.to_string())
}

/// 50ms 대기가 있는 붙여넣기이므로 별도 스레드에서 수행하고 바로 돌아온다.
#[tauri::command]
pub fn paste_clip(app: AppHandle, id: i64) {
    std::thread::spawn(move || {
        if let Err(e) = crate::clipboard::popup::paste(&app, id) {
            log::warn!("붙여넣기 실패: {e:#}");
        }
    });
}

#[tauri::command]
pub fn hide_clipboard_popup(app: AppHandle) -> Result<(), String> {
    crate::clipboard::popup::hide(&app).map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_with_duplicate_hotkey_is_rejected_and_keeps_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path().join("settings.json"));
        let current = Settings::default();
        store.save(&current).unwrap();

        let mut bad = Settings::default();
        bad.window_hotkeys[0].hotkey = bad.clipboard.hotkey.clone();
        let src = dir.path().join("in.json");
        store.export_to(&bad, &src).unwrap();

        assert!(!import_validated(&store, &src).unwrap_err().is_empty());
        assert_eq!(store.load(), current);

        store.export_to(&current, &src).unwrap();
        assert_eq!(import_validated(&store, &src).unwrap(), current);
    }
}
