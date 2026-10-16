use std::sync::RwLock;

use crate::settings::model::Settings;
use crate::settings::store::SettingsStore;

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub store: SettingsStore,
}
