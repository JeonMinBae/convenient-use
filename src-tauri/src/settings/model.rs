use serde::{Deserialize, Serialize};

use crate::remap::RemapRule;
use crate::window_layout::LayoutCommand;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub general: GeneralSettings,
    pub launch_items: Vec<LaunchItem>,
    pub window_hotkeys: Vec<WindowHotkey>,
    pub clipboard: ClipboardSettings,
    pub snippets: Vec<Snippet>,
    pub remap: RemapSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            launch_items: Vec::new(),
            window_hotkeys: default_window_hotkeys(),
            clipboard: ClipboardSettings::default(),
            snippets: Vec::new(),
            remap: RemapSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralSettings {
    pub autostart: bool,
    pub features: FeatureToggles,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self { autostart: true, features: FeatureToggles::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FeatureToggles {
    pub launch: bool,
    pub window: bool,
    pub clipboard: bool,
    pub snippet: bool,
    pub remap: bool,
}

impl Default for FeatureToggles {
    fn default() -> Self {
        Self { launch: true, window: true, clipboard: true, snippet: true, remap: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchItem {
    pub id: String,
    pub name: String,
    pub hotkey: String,
    pub target: LaunchTarget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LaunchTarget {
    App { path: OsPath },
    // browser 미지정이면 기본 브라우저
    Url {
        url: String,
        #[serde(default)]
        browser: OsPath,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowHotkey {
    pub command: LayoutCommand,
    pub hotkey: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipboardSettings {
    pub hotkey: String,
    pub max_items: u32,
    pub max_image_mb: u32,
}

impl Default for ClipboardSettings {
    fn default() -> Self {
        let hotkey = if cfg!(target_os = "macos") { "Shift+Super+KeyV" } else { "Ctrl+Shift+KeyV" };
        Self { hotkey: hotkey.into(), max_items: 500, max_image_mb: 10 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub keyword: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RemapSettings {
    pub rules: Vec<RemapRule>,
    // 실행 파일 이름, 소문자 비교 (예: "valorant.exe")
    pub excluded_apps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OsPath {
    pub windows: Option<String>,
    pub macos: Option<String>,
}

impl OsPath {
    pub fn current(&self) -> Option<&str> {
        let v = if cfg!(windows) { &self.windows } else { &self.macos };
        v.as_deref().filter(|s| !s.is_empty())
    }
}

fn default_window_hotkeys() -> Vec<WindowHotkey> {
    use LayoutCommand::*;
    [
        (LeftHalf, "Ctrl+Alt+ArrowLeft"), (RightHalf, "Ctrl+Alt+ArrowRight"),
        (TopHalf, "Ctrl+Alt+ArrowUp"), (BottomHalf, "Ctrl+Alt+ArrowDown"),
        (TopLeft, "Ctrl+Alt+KeyU"), (TopRight, "Ctrl+Alt+KeyI"),
        (BottomLeft, "Ctrl+Alt+KeyJ"), (BottomRight, "Ctrl+Alt+KeyK"),
        (FirstThird, "Ctrl+Alt+KeyD"), (CenterThird, "Ctrl+Alt+KeyF"), (LastThird, "Ctrl+Alt+KeyG"),
        (FirstTwoThirds, "Ctrl+Alt+KeyE"), (LastTwoThirds, "Ctrl+Alt+KeyT"),
        (Maximize, "Ctrl+Alt+Enter"), (Center, "Ctrl+Alt+KeyC"),
        (NextDisplay, "Ctrl+Alt+Super+ArrowRight"),
    ]
    .into_iter()
    .map(|(command, hotkey)| WindowHotkey { command, hotkey: hotkey.into() })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
        assert!(s.general.autostart);
        assert_eq!(s.clipboard.max_items, 500);
        assert_eq!(s.clipboard.max_image_mb, 10);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let s: Settings = serde_json::from_str(r#"{"clipboard":{"maxItems":50}}"#).unwrap();
        assert_eq!(s.clipboard.max_items, 50);
        assert_eq!(s.clipboard.max_image_mb, 10);
        assert_eq!(s.window_hotkeys.len(), 16);
    }

    #[test]
    fn other_os_path_is_not_current() {
        let p = OsPath { windows: None, macos: Some("/Applications/Slack.app".into()) };
        if cfg!(windows) {
            assert_eq!(p.current(), None)
        }
    }

    #[test]
    fn all_features_on_by_default() {
        let f = FeatureToggles::default();
        assert!(f.launch && f.window && f.clipboard && f.snippet && f.remap);
    }
}
