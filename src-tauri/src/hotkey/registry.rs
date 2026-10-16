use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use super::accelerator::{AccelError, normalize};
use crate::remap::RemapRule;
use crate::settings::model::Settings;
use crate::state::AppState;
use crate::window_layout::LayoutCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum BindingSource {
    Launch(String),
    Window(LayoutCommand),
    Clipboard,
    Macro(usize),
}

impl BindingSource {
    /// 프론트 폼 필드를 가리키는 이름 (ValidationError.field 와 동일 규칙)
    pub fn field(&self) -> String {
        match self {
            Self::Launch(id) => format!("launchItems.{id}.hotkey"),
            Self::Window(cmd) => {
                let name = serde_json::to_value(cmd)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                format!("windowHotkeys.{name}")
            }
            Self::Clipboard => "clipboard.hotkey".into(),
            Self::Macro(i) => format!("remap.rules.{i}.trigger"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub source: BindingSource,
    pub accel: String,
}

/// 형식이 잘못된 단축키 (원문 보존)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidBinding {
    pub source: BindingSource,
    pub raw: String,
    pub error: AccelError,
}

/// 활성 기능의 바인딩만 `Launch → Window → Clipboard → Macro` 순으로 모은다.
/// 비어 있는 단축키는 "미지정"으로 보고 건너뛴다. 형식 오류는 따로 모아 돌려준다.
pub fn collect(s: &Settings) -> (Vec<Binding>, Vec<InvalidBinding>) {
    let f = &s.general.features;
    let mut raw: Vec<(BindingSource, &str)> = Vec::new();
    if f.launch {
        raw.extend(s.launch_items.iter().map(|i| (BindingSource::Launch(i.id.clone()), i.hotkey.as_str())));
    }
    if f.window {
        raw.extend(s.window_hotkeys.iter().map(|w| (BindingSource::Window(w.command), w.hotkey.as_str())));
    }
    if f.clipboard {
        raw.push((BindingSource::Clipboard, s.clipboard.hotkey.as_str()));
    }
    if f.remap {
        for (i, rule) in s.remap.rules.iter().enumerate() {
            if let RemapRule::Macro { trigger, .. } = rule {
                raw.push((BindingSource::Macro(i), trigger.as_str()));
            }
        }
    }

    let (mut ok, mut bad) = (Vec::new(), Vec::new());
    for (source, text) in raw.into_iter().filter(|(_, t)| !t.trim().is_empty()) {
        match normalize(text) {
            Ok(accel) => ok.push(Binding { source, accel }),
            Err(error) => bad.push(InvalidBinding { source, raw: text.to_string(), error }),
        }
    }
    (ok, bad)
}

#[cfg(test)]
pub fn collect_bindings(s: &Settings) -> Result<Vec<Binding>, Vec<InvalidBinding>> {
    let (ok, bad) = collect(s);
    if bad.is_empty() { Ok(ok) } else { Err(bad) }
}

/// 같은 정규화 문자열을 가진 바인딩 묶음(2개 이상)을 돌려준다.
pub fn find_conflicts(bindings: &[Binding]) -> Vec<Vec<BindingSource>> {
    let mut map: BTreeMap<&str, Vec<BindingSource>> = BTreeMap::new();
    for b in bindings {
        map.entry(&b.accel).or_default().push(b.source.clone());
    }
    map.into_values().filter(|v| v.len() > 1).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    LaunchItem(String),
    Layout(LayoutCommand),
    ClipboardPopup,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyFailure {
    pub source: BindingSource,
    pub hotkey: String,
    pub reason: String,
}

/// 실제 OS 등록 대상. 매크로는 리매핑이 처리한다.
pub fn registrations(bindings: &[Binding]) -> Vec<(Action, &Binding)> {
    bindings
        .iter()
        .filter_map(|b| match &b.source {
            BindingSource::Launch(id) => Some((Action::LaunchItem(id.clone()), b)),
            BindingSource::Window(cmd) => Some((Action::Layout(*cmd), b)),
            BindingSource::Clipboard => Some((Action::ClipboardPopup, b)),
            BindingSource::Macro(_) => None,
        })
        .collect()
}

#[derive(Default)]
pub struct HotkeyManager {
    by_id: HashMap<u32, Action>,
    failures: Vec<HotkeyFailure>,
    paused: bool,
}

impl HotkeyManager {
    pub fn failures(&self) -> Vec<HotkeyFailure> {
        self.failures.clone()
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// 전체 해제 후 설정대로 다시 등록한다. 일시정지 중이면 해제만 한다.
    pub fn reload(&mut self, app: &AppHandle, settings: &Settings) {
        let gs = app.global_shortcut();
        if let Err(e) = gs.unregister_all() {
            log::warn!("단축키 해제 실패: {e}");
        }
        self.by_id.clear();
        self.failures.clear();
        if self.paused {
            return;
        }

        let (bindings, invalid) = collect(settings);
        self.failures.extend(invalid.into_iter().map(|b| HotkeyFailure {
            source: b.source,
            hotkey: b.raw,
            reason: b.error.to_string(),
        }));
        for (action, b) in registrations(&bindings) {
            let result = Shortcut::from_str(&b.accel)
                .map_err(|e| e.to_string())
                .and_then(|sc| gs.register(sc).map(|_| sc.id()).map_err(|e| e.to_string()));
            match result {
                Ok(id) => {
                    self.by_id.insert(id, action);
                }
                Err(reason) => self.failures.push(HotkeyFailure {
                    source: b.source.clone(),
                    hotkey: b.accel.clone(),
                    reason,
                }),
            }
        }
    }

    fn action_for(&self, id: u32) -> Option<Action> {
        if self.paused { None } else { self.by_id.get(&id).cloned() }
    }
}

/// 플러그인 핸들러에서 호출한다. 핸들러 스레드를 막지 않도록 실제 동작은 별도 스레드에서 수행한다.
pub fn dispatch(app: &AppHandle, id: u32) {
    let state = app.state::<AppState>();
    let Some(action) = state.hotkeys.lock().unwrap().action_for(id) else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let result = match action {
            Action::LaunchItem(item_id) => {
                let item = app
                    .state::<AppState>()
                    .settings
                    .read()
                    .unwrap()
                    .launch_items
                    .iter()
                    .find(|i| i.id == item_id)
                    .cloned();
                match item {
                    Some(item) => crate::launcher::run(&app, &item),
                    None => Ok(()),
                }
            }
            Action::Layout(cmd) => crate::window_layout::apply(cmd),
            Action::ClipboardPopup => crate::clipboard::popup::show(&app),
        };
        if let Err(e) = result {
            log::warn!("단축키 동작 실패: {e:#}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::{LaunchItem, LaunchTarget, OsPath};

    fn sorted(mut v: Vec<BindingSource>) -> Vec<BindingSource> {
        let rank = |s: &BindingSource| match s {
            BindingSource::Launch(_) => 0,
            BindingSource::Window(_) => 1,
            BindingSource::Clipboard => 2,
            BindingSource::Macro(_) => 3,
        };
        v.sort_by_key(rank);
        v
    }

    fn launch(id: &str, hotkey: &str) -> LaunchItem {
        LaunchItem {
            id: id.into(),
            name: id.into(),
            hotkey: hotkey.into(),
            target: LaunchTarget::Url { url: "https://x".into(), browser: OsPath::default() },
        }
    }

    #[test]
    fn defaults_have_no_conflicts() {
        let b = collect_bindings(&Settings::default()).unwrap();
        assert!(find_conflicts(&b).is_empty());
        assert_eq!(b.len(), 17); // 창 16 + 클립보드
    }

    #[test]
    fn conflict_detected_across_features() {
        let mut s = Settings::default();
        s.launch_items.push(launch("slack", "alt+ctrl+left"));
        let conflicts = find_conflicts(&collect_bindings(&s).unwrap());
        assert_eq!(
            conflicts,
            vec![sorted(vec![
                BindingSource::Launch("slack".into()),
                BindingSource::Window(LayoutCommand::LeftHalf),
            ])]
        );
    }

    #[test]
    fn disabled_feature_is_not_collected() {
        let mut s = Settings::default();
        s.general.features.window = false;
        assert!(collect_bindings(&s).unwrap().iter().all(|b| !matches!(b.source, BindingSource::Window(_))));
        // 꺼진 기능과 겹치는 단축키는 충돌이 아니다
        s.launch_items.push(launch("slack", "alt+ctrl+left"));
        assert!(find_conflicts(&collect_bindings(&s).unwrap()).is_empty());
    }

    #[test]
    fn macro_trigger_collected_and_invalid_reported() {
        let mut s = Settings::default();
        s.remap.rules.push(RemapRule::Macro { trigger: "ctrl+shift+v".into(), steps: vec![] });
        s.launch_items.push(launch("bad", "Ctrl+Shift"));
        s.launch_items.push(launch("none", " "));
        let (ok, bad) = collect(&s);
        assert_eq!(find_conflicts(&ok).len(), 1); // 클립보드 기본값과 매크로
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].source, BindingSource::Launch("bad".into()));
    }

    #[test]
    fn clipboard_registered_only_when_feature_on() {
        let mut s = Settings::default();
        let b = collect_bindings(&s).unwrap();
        assert_eq!(registrations(&b).len(), 17);
        assert!(registrations(&b).iter().any(|(a, _)| *a == Action::ClipboardPopup));
        s.general.features.clipboard = false;
        assert_eq!(registrations(&collect_bindings(&s).unwrap()).len(), 16);
    }

    #[test]
    fn field_names() {
        assert_eq!(BindingSource::Window(LayoutCommand::LeftHalf).field(), "windowHotkeys.leftHalf");
        assert_eq!(BindingSource::Macro(2).field(), "remap.rules.2.trigger");
    }
}
