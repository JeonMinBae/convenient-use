use std::path::Path;

use tauri::State;

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
pub fn import_settings(state: State<'_, AppState>, path: String) -> Result<Settings, String> {
    let imported = state.store.import_from(Path::new(&path)).map_err(|e| format!("{e:#}"))?;
    *state.settings.write().unwrap() = imported.clone();
    Ok(imported)
}
