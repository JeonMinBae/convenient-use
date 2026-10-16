use std::sync::{Mutex, RwLock};

use crate::hotkey::registry::HotkeyManager;
use crate::settings::model::Settings;
use crate::settings::store::SettingsStore;

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub store: SettingsStore,
    pub hotkeys: Mutex<HotkeyManager>,
}
