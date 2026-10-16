use std::sync::{Mutex, RwLock};
use std::time::Instant;

use crate::clipboard::store::ClipStore;

use crate::hotkey::registry::HotkeyManager;
use crate::settings::model::Settings;
use crate::settings::store::SettingsStore;

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub store: SettingsStore,
    pub hotkeys: Mutex<HotkeyManager>,
    pub clips: Mutex<ClipStore>,
    /// 이 시각 이전의 클립보드 변경은 감시기가 기록하지 않는다
    pub clip_suppress_until: Mutex<Option<Instant>>,
}
