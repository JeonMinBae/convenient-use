use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::hotkey::accelerator::normalize;
use crate::keyboard::keys::code_to_vk;
use crate::keyboard::{KeyDecision, KeyEvent, KeyHandler};
use crate::platform::current::{input, process, window_ops};
use crate::settings::model::RemapSettings;
use crate::settings::validate::ValidationError;

// mods: "Ctrl" | "Alt" | "Shift" | "Super"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyCombo {
    pub mods: Vec<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RemapRule {
    Simple { from: String, to: KeyCombo },
    TapHold { key: String, tap: KeyCombo, hold: KeyCombo },
    Layer { trigger: String, tap: Option<KeyCombo>, mappings: Vec<LayerMapping> },
    // trigger 는 단축키 문자열
    Macro { trigger: String, steps: Vec<KeyCombo> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerMapping {
    pub from: String,
    pub to: KeyCombo,
}

// 키 리매핑 엔진. `Engine` 은 OS 호출 없는 상태 기계이고, `RemapHandler` 가 훅 이벤트를 엔진에 넣어 출력을 주입한다.

const TAP_TIMEOUT_MS: u64 = 200;
const MODS: [&str; 4] = ["Ctrl", "Alt", "Shift", "Super"];

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub suppress: bool,
    /// (code, down) 순서대로 주입한다
    pub emit: Vec<(String, bool)>,
}

impl Output {
    pub fn suppress(emit: Vec<(String, bool)>) -> Self {
        Self { suppress: true, emit }
    }
    pub fn pass() -> Self {
        Self { suppress: false, emit: vec![] }
    }
}

fn mod_code(m: &str) -> Option<&'static str> {
    Some(match m {
        "Ctrl" => "ControlLeft",
        "Alt" => "AltLeft",
        "Shift" => "ShiftLeft",
        "Super" => "MetaLeft",
        _ => return None,
    })
}

/// 좌우 구분 없이 수정자 이름으로 (수정자 키가 아니면 None)
fn mod_of(code: &str) -> Option<&'static str> {
    Some(match code {
        "ControlLeft" | "ControlRight" => "Ctrl",
        "AltLeft" | "AltRight" => "Alt",
        "ShiftLeft" | "ShiftRight" => "Shift",
        "MetaLeft" | "MetaRight" => "Super",
        _ => return None,
    })
}

/// down: 수정자 순서대로 누른 뒤 키. up: 키를 뗀 뒤 수정자를 역순으로.
fn combo_edge(c: &KeyCombo, down: bool) -> Vec<(String, bool)> {
    let mods = c.mods.iter().filter_map(|m| mod_code(m)).map(|m| (m.to_string(), down));
    let key = c.key.iter().map(|k| (k.clone(), down));
    if down {
        mods.chain(key).collect()
    } else {
        key.chain(mods.rev()).collect()
    }
}

fn combo_tap(c: &KeyCombo) -> Vec<(String, bool)> {
    let mut v = combo_edge(c, true);
    v.extend(combo_edge(c, false));
    v
}

enum Pending {
    TapHold { key: String, tap: KeyCombo, hold: KeyCombo, since: u64, held: bool },
    Layer { trigger: String, tap: Option<KeyCombo>, since: u64, used: bool },
}

struct LayerDef {
    tap: Option<KeyCombo>,
    mappings: HashMap<String, KeyCombo>,
}

struct MacroDef {
    mods: HashSet<String>,
    key: String,
    steps: Vec<KeyCombo>,
}

pub struct Engine {
    simple: HashMap<String, KeyCombo>,
    tap_hold: HashMap<String, (KeyCombo, KeyCombo)>,
    layers: HashMap<String, LayerDef>,
    macros: Vec<MacroDef>,
    pending: Option<Pending>,
    /// 눌림이 진행 중인 Simple/Layer 매핑 (키 → 출력). up 때 같은 출력을 뗀다
    active: HashMap<String, KeyCombo>,
    /// 매크로가 삼킨 트리거 키 (up 과 자동 반복 down 을 억제)
    swallow_up: HashSet<String>,
    /// 물리 수정자 키 (엔진에 들어온 이벤트로 추적)
    held_mods: HashSet<String>,
    /// 엔진이 지금 눌러 둔 출력 키. reset 시 모두 뗀다
    out_down: Vec<String>,
}

impl Engine {
    pub fn new(rules: &[RemapRule]) -> Self {
        let mut e = Self {
            simple: HashMap::new(),
            tap_hold: HashMap::new(),
            layers: HashMap::new(),
            macros: vec![],
            pending: None,
            active: HashMap::new(),
            swallow_up: HashSet::new(),
            held_mods: HashSet::new(),
            out_down: vec![],
        };
        for r in rules {
            match r {
                RemapRule::Simple { from, to } => {
                    e.simple.insert(from.clone(), to.clone());
                }
                RemapRule::TapHold { key, tap, hold } => {
                    e.tap_hold.insert(key.clone(), (tap.clone(), hold.clone()));
                }
                RemapRule::Layer { trigger, tap, mappings } => {
                    let mappings = mappings.iter().map(|m| (m.from.clone(), m.to.clone())).collect();
                    e.layers.insert(trigger.clone(), LayerDef { tap: tap.clone(), mappings });
                }
                RemapRule::Macro { trigger, steps } => {
                    // 형식이 잘못된 트리거는 검증에서 걸러진다. 여기서는 무시만 한다
                    let Ok(norm) = normalize(trigger) else { continue };
                    let mut parts: Vec<&str> = norm.split('+').collect();
                    let key = parts.pop().unwrap_or_default().to_string();
                    let mods = parts.into_iter().map(String::from).collect();
                    e.macros.push(MacroDef { mods, key, steps: steps.clone() });
                }
            }
        }
        e
    }

    /// 진행 중 상태를 버리고, 엔진이 눌러 둔 출력 키를 떼는 이벤트를 돌려준다 (키 고착 방지).
    pub fn reset(&mut self) -> Vec<(String, bool)> {
        self.pending = None;
        self.active.clear();
        self.swallow_up.clear();
        self.held_mods.clear();
        std::mem::take(&mut self.out_down).into_iter().rev().map(|c| (c, false)).collect()
    }

    fn out(&mut self, suppress: bool, emit: Vec<(String, bool)>) -> Output {
        for (code, down) in &emit {
            self.out_down.retain(|c| c != code);
            if *down {
                self.out_down.push(code.clone());
            }
        }
        Output { suppress, emit }
    }

    pub fn process(&mut self, ev: &KeyEvent) -> Output {
        if mod_of(&ev.code).is_some() {
            if ev.down {
                self.held_mods.insert(ev.code.clone());
            } else {
                self.held_mods.remove(&ev.code);
            }
        }
        if let Some(out) = self.continue_pending(ev) {
            return out;
        }
        if !ev.down {
            if let Some(to) = self.active.remove(&ev.code) {
                return self.out(true, combo_edge(&to, false));
            }
            if self.swallow_up.remove(&ev.code) {
                return Output::suppress(vec![]);
            }
        } else {
            if self.swallow_up.contains(&ev.code) {
                return Output::suppress(vec![]);
            }
            if let Some(emit) = self.match_macro(ev) {
                self.swallow_up.insert(ev.code.clone());
                // 균형 잡힌 이벤트(물리 수정자 해제→스텝→복원)라 out_down 에는 기록하지 않는다
                return Output::suppress(emit);
            }
        }
        if let Some(to) = self.simple.get(&ev.code).cloned() {
            if ev.down {
                self.active.insert(ev.code.clone(), to.clone());
            }
            return self.out(true, combo_edge(&to, ev.down));
        }
        if ev.down && self.pending.is_none()
            && let Some(p) = self.start_pending(ev) {
                self.pending = Some(p);
                return Output::suppress(vec![]);
            }
        Output::pass()
    }

    /// 매크로는 사용자가 누르고 있는 물리 모디파이어 위에 주입되므로, 먼저 떼고 끝난 뒤 다시 누른다.
    fn match_macro(&self, ev: &KeyEvent) -> Option<Vec<(String, bool)>> {
        let held: HashSet<&str> = self.held_mods.iter().filter_map(|c| mod_of(c)).collect();
        let m = self.macros.iter().find(|m| {
            m.key == ev.code && m.mods.len() == held.len() && m.mods.iter().all(|x| held.contains(x.as_str()))
        })?;
        let mut physical: Vec<&String> = self.held_mods.iter().collect();
        physical.sort();
        let mut emit: Vec<(String, bool)> = physical.iter().map(|c| ((*c).clone(), false)).collect();
        for step in &m.steps {
            emit.extend(combo_tap(step));
        }
        emit.extend(physical.iter().map(|c| ((*c).clone(), true)));
        Some(emit)
    }

    fn start_pending(&self, ev: &KeyEvent) -> Option<Pending> {
        if let Some((tap, hold)) = self.tap_hold.get(&ev.code) {
            return Some(Pending::TapHold {
                key: ev.code.clone(),
                tap: tap.clone(),
                hold: hold.clone(),
                since: ev.time_ms,
                held: false,
            });
        }
        let l = self.layers.get(&ev.code)?;
        Some(Pending::Layer { trigger: ev.code.clone(), tap: l.tap.clone(), since: ev.time_ms, used: false })
    }

    /// TapHold/Layer 진행 중 처리: 다른 키 down → hold(레이어 사용) 확정,
    /// 트리거 up → 200ms 미만·미사용이면 tap, 확정됐으면 hold up, 그 외는 출력 없음.
    fn continue_pending(&mut self, ev: &KeyEvent) -> Option<Output> {
        match self.pending.as_mut()? {
            Pending::TapHold { key, tap, hold, since, held } => {
                if ev.code == *key {
                    if ev.down {
                        return Some(Output::suppress(vec![])); // 자동 반복
                    }
                    let emit = if *held {
                        combo_edge(hold, false)
                    } else if ev.time_ms.saturating_sub(*since) < TAP_TIMEOUT_MS {
                        combo_tap(tap)
                    } else {
                        vec![]
                    };
                    self.pending = None;
                    return Some(self.out(true, emit));
                }
                if ev.down && !*held {
                    *held = true;
                    let emit = combo_edge(hold, true);
                    return Some(self.out(false, emit));
                }
                None
            }
            Pending::Layer { trigger, tap, since, used } => {
                if ev.code == *trigger {
                    if ev.down {
                        return Some(Output::suppress(vec![]));
                    }
                    let emit = match tap {
                        Some(t) if !*used && ev.time_ms.saturating_sub(*since) < TAP_TIMEOUT_MS => combo_tap(t),
                        _ => vec![],
                    };
                    self.pending = None;
                    return Some(self.out(true, emit));
                }
                if !ev.down {
                    return None;
                }
                *used = true;
                let to = self.layers.get(trigger.as_str())?.mappings.get(&ev.code)?.clone();
                self.active.insert(ev.code.clone(), to.clone());
                Some(self.out(true, combo_edge(&to, true)))
            }
        }
    }
}

/// 훅 스레드에서 호출된다. 출력은 반환 전에 동기로 주입해 키 순서를 보존한다:
/// 주입 이벤트는 INJECTED_MARK 로 `keyboard::on_event` 가 락 없이 통과시키므로 재진입해도 교착하지 않는다.
pub struct RemapHandler {
    engine: Engine,
    excluded: HashSet<String>,
    /// 마지막으로 확인한 포그라운드 창과 제외 여부 (창이 바뀔 때만 exe 를 조회해 훅을 빠르게 유지)
    fg: Option<(Option<isize>, bool)>,
}

impl RemapHandler {
    pub fn new(s: &RemapSettings) -> Self {
        Self {
            engine: Engine::new(&s.rules),
            excluded: s.excluded_apps.iter().map(|a| a.trim().to_lowercase()).filter(|a| !a.is_empty()).collect(),
            fg: None,
        }
    }

    fn is_excluded(&mut self) -> bool {
        if self.excluded.is_empty() {
            return false;
        }
        let hwnd = window_ops::foreground().ok();
        if let Some((h, ex)) = self.fg
            && h == hwnd {
                return ex;
            }
        let ex = process::foreground_exe().is_ok_and(|e| self.excluded.contains(&e));
        self.fg = Some((hwnd, ex));
        ex
    }

    fn inject(emit: &[(String, bool)]) {
        if emit.is_empty() {
            return;
        }
        if let Err(e) = input::send_events(emit) {
            log::warn!("리매핑 출력 주입 실패: {e}");
        }
    }
}

impl KeyHandler for RemapHandler {
    fn on_key(&mut self, ev: &KeyEvent) -> KeyDecision {
        if self.is_excluded() {
            // 제외 앱에서는 통과. 엔진이 눌러 둔 키가 있으면 떼고 상태를 비운다 (없으면 빈 이벤트)
            Self::inject(&self.engine.reset());
            return KeyDecision::Pass;
        }
        let out = self.engine.process(ev);
        Self::inject(&out.emit);
        if out.suppress { KeyDecision::Suppress } else { KeyDecision::Pass }
    }

    fn release(&mut self) {
        Self::inject(&self.engine.reset());
    }
}

fn combo_error(c: &KeyCombo) -> Option<String> {
    if c.mods.is_empty() && c.key.is_none() {
        return Some("키를 지정하세요".into());
    }
    if let Some(m) = c.mods.iter().find(|m| !MODS.contains(&m.as_str())) {
        return Some(format!("알 수 없는 수정자입니다: {m}"));
    }
    match &c.key {
        Some(k) if code_to_vk(k).is_none() => Some(format!("지원하지 않는 키입니다: {k}")),
        _ => None,
    }
}

fn key_error(k: &str) -> Option<String> {
    if k.is_empty() {
        Some("키를 지정하세요".into())
    } else if code_to_vk(k).is_none() {
        Some(format!("지원하지 않는 키입니다: {k}"))
    } else {
        None
    }
}

/// 리매핑 규칙 검증. 필드 이름은 `remap.rules.{i}.{from|key|trigger|to|tap|hold|steps|steps.{j}|mappings.{j}.from|mappings.{j}.to}`.
pub fn validate_rules(s: &RemapSettings) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut add = |field: String, message: String| errors.push(ValidationError { field, message });
    let mut triggers: HashMap<&str, Vec<String>> = HashMap::new();
    for (i, r) in s.rules.iter().enumerate() {
        let f = |name: &str| format!("remap.rules.{i}.{name}");
        match r {
            RemapRule::Simple { from, to } => {
                match key_error(from) {
                    Some(m) => add(f("from"), m),
                    None => triggers.entry(from).or_default().push(f("from")),
                }
                if let Some(m) = combo_error(to) {
                    add(f("to"), m);
                }
            }
            RemapRule::TapHold { key, tap, hold } => {
                match key_error(key) {
                    Some(m) => add(f("key"), m),
                    None => triggers.entry(key).or_default().push(f("key")),
                }
                if let Some(m) = combo_error(tap) {
                    add(f("tap"), m);
                }
                if let Some(m) = combo_error(hold) {
                    add(f("hold"), m);
                }
            }
            RemapRule::Layer { trigger, tap, mappings } => {
                match key_error(trigger) {
                    Some(m) => add(f("trigger"), m),
                    None => triggers.entry(trigger).or_default().push(f("trigger")),
                }
                if let Some(m) = tap.as_ref().and_then(combo_error) {
                    add(f("tap"), m);
                }
                for (j, m) in mappings.iter().enumerate() {
                    if let Some(msg) = key_error(&m.from) {
                        add(f(&format!("mappings.{j}.from")), msg);
                    } else if mappings.iter().enumerate().any(|(k, o)| k != j && o.from == m.from) {
                        add(f(&format!("mappings.{j}.from")), "같은 키가 중복되었습니다".into());
                    }
                    if let Some(msg) = combo_error(&m.to) {
                        add(f(&format!("mappings.{j}.to")), msg);
                    }
                }
            }
            RemapRule::Macro { trigger, steps } => {
                // 형식·전역 단축키 충돌은 hotkey::registry 검사가 같은 필드 이름으로 보고한다
                if trigger.trim().is_empty() {
                    add(f("trigger"), "단축키를 입력하세요".into());
                }
                if steps.is_empty() {
                    add(f("steps"), "실행할 키를 하나 이상 추가하세요".into());
                }
                for (j, c) in steps.iter().enumerate() {
                    if let Some(m) = combo_error(c) {
                        add(f(&format!("steps.{j}")), m);
                    }
                }
            }
        }
    }
    for fields in triggers.into_values().filter(|v| v.len() > 1) {
        for field in &fields {
            add(field.clone(), "다른 규칙이 같은 키를 사용합니다".into());
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(mods: &[&str], key: &str) -> KeyCombo {
        KeyCombo { mods: mods.iter().map(|s| s.to_string()).collect(), key: Some(key.into()) }
    }
    fn mods(m: &[&str]) -> Vec<String> {
        m.iter().map(|s| s.to_string()).collect()
    }
    fn at(code: &str, down: bool, t: u64) -> KeyEvent {
        KeyEvent { time_ms: t, ..KeyEvent::key(code, down) }
    }
    fn mod_down(code: &str) -> KeyEvent {
        KeyEvent::key(code, true)
    }
    /// 매크로 앞뒤의 물리 모디파이어 해제·복원 이벤트(각 n개)를 제거한다
    fn strip_restore(emit: &[(String, bool)], n: usize) -> Vec<(String, bool)> {
        emit[n..emit.len() - n].to_vec()
    }
    fn ev(s: &str, down: bool) -> (String, bool) {
        (s.to_string(), down)
    }

    fn caps_tap_hold() -> Engine {
        Engine::new(&[RemapRule::TapHold {
            key: "CapsLock".into(),
            tap: KeyCombo { mods: vec![], key: Some("Escape".into()) },
            hold: KeyCombo { mods: vec!["Ctrl".into()], key: None },
        }])
    }

    #[test]
    fn simple_maps_down_and_up() {
        let mut e = Engine::new(&[RemapRule::Simple { from: "CapsLock".into(), to: combo(&[], "Escape") }]);
        assert_eq!(e.process(&at("CapsLock", true, 0)), Output::suppress(vec![ev("Escape", true)]));
        assert_eq!(e.process(&at("CapsLock", false, 10)), Output::suppress(vec![ev("Escape", false)]));
        assert!(!e.process(&at("KeyA", true, 20)).suppress);
    }

    #[test]
    fn hyper_key_presses_all_modifiers_in_order() {
        let to = KeyCombo { mods: mods(&["Ctrl", "Alt", "Shift", "Super"]), key: None };
        let mut e = Engine::new(&[RemapRule::Simple { from: "CapsLock".into(), to }]);
        let down = e.process(&at("CapsLock", true, 0)).emit;
        assert_eq!(down, vec![ev("ControlLeft", true), ev("AltLeft", true), ev("ShiftLeft", true), ev("MetaLeft", true)]);
        let up = e.process(&at("CapsLock", false, 5)).emit;
        assert_eq!(up.first(), Some(&ev("MetaLeft", false)));
        assert_eq!(up.last(), Some(&ev("ControlLeft", false)));
    }

    #[test]
    fn tap_within_timeout_sends_tap() {
        let mut e = caps_tap_hold();
        assert!(e.process(&at("CapsLock", true, 0)).emit.is_empty());
        assert_eq!(e.process(&at("CapsLock", false, 120)).emit, vec![ev("Escape", true), ev("Escape", false)]);
    }

    #[test]
    fn other_key_while_pending_resolves_hold() {
        let mut e = caps_tap_hold();
        e.process(&at("CapsLock", true, 0));
        let out = e.process(&at("KeyC", true, 50));
        assert!(!out.suppress, "KeyC 자체는 통과");
        assert_eq!(out.emit, vec![ev("ControlLeft", true)]);
        e.process(&at("KeyC", false, 60));
        assert_eq!(e.process(&at("CapsLock", false, 300)).emit, vec![ev("ControlLeft", false)]);
    }

    #[test]
    fn long_press_alone_sends_nothing() {
        let mut e = caps_tap_hold();
        e.process(&at("CapsLock", true, 0));
        assert!(e.process(&at("CapsLock", false, 500)).emit.is_empty());
    }

    fn caps_layer() -> Engine {
        Engine::new(&[RemapRule::Layer {
            trigger: "CapsLock".into(),
            tap: Some(combo(&[], "Escape")),
            mappings: vec![LayerMapping { from: "KeyH".into(), to: combo(&[], "ArrowLeft") }],
        }])
    }

    #[test]
    fn layer_maps_keys_while_trigger_held() {
        let mut e = caps_layer();
        e.process(&at("CapsLock", true, 0));
        assert_eq!(e.process(&at("KeyH", true, 30)), Output::suppress(vec![ev("ArrowLeft", true)]));
        assert_eq!(e.process(&at("KeyH", false, 40)), Output::suppress(vec![ev("ArrowLeft", false)]));
        assert!(e.process(&at("CapsLock", false, 60)).emit.is_empty(), "레이어를 썼으면 tap 안 보냄");
        assert!(!e.process(&at("KeyH", true, 70)).suppress, "트리거 뗀 뒤에는 원래 키");
    }

    #[test]
    fn layer_tap_and_late_release_of_mapped_key() {
        let mut e = caps_layer();
        e.process(&at("CapsLock", true, 0));
        assert_eq!(e.process(&at("CapsLock", false, 80)).emit, vec![ev("Escape", true), ev("Escape", false)]);
        e.process(&at("CapsLock", true, 100));
        e.process(&at("KeyH", true, 120));
        e.process(&at("CapsLock", false, 200));
        assert_eq!(e.process(&at("KeyH", false, 210)).emit, vec![ev("ArrowLeft", false)], "트리거를 먼저 떼도 출력 키는 해제");
    }

    fn macro_engine() -> Engine {
        Engine::new(&[RemapRule::Macro {
            trigger: "Ctrl+Alt+KeyM".into(),
            steps: vec![combo(&["Ctrl"], "KeyA"), combo(&["Ctrl"], "KeyC")],
        }])
    }

    #[test]
    fn macro_sends_sequence_and_swallows_trigger_up() {
        let mut e = macro_engine();
        e.process(&mod_down("ControlLeft"));
        e.process(&mod_down("AltLeft"));
        let out = e.process(&at("KeyM", true, 0));
        assert!(out.suppress);
        assert_eq!(
            strip_restore(&out.emit, 2),
            vec![
                ev("ControlLeft", true), ev("KeyA", true), ev("KeyA", false), ev("ControlLeft", false),
                ev("ControlLeft", true), ev("KeyC", true), ev("KeyC", false), ev("ControlLeft", false),
            ]
        );
        assert!(out.emit[..2].iter().all(|(_, d)| !d) && out.emit[out.emit.len() - 2..].iter().all(|(_, d)| *d));
        assert!(e.process(&at("KeyM", true, 5)).emit.is_empty(), "자동 반복은 재실행 안 함");
        assert!(e.process(&at("KeyM", false, 10)).suppress);
        e.process(&at("ControlLeft", false, 15));
        e.process(&at("AltLeft", false, 16));
        assert!(!e.process(&at("KeyM", true, 20)).suppress, "수정자 없이는 매크로 아님");
    }

    #[test]
    fn macro_requires_exact_modifiers() {
        let mut e = macro_engine();
        e.process(&mod_down("ControlLeft"));
        assert!(!e.process(&at("KeyM", true, 0)).suppress);
    }

    #[test]
    fn reset_releases_held_outputs() {
        let mut e = caps_tap_hold();
        e.process(&at("CapsLock", true, 0));
        e.process(&at("KeyC", true, 50)); // hold 확정: ControlLeft down
        assert_eq!(e.reset(), vec![ev("ControlLeft", false)]);
        assert!(e.reset().is_empty());
        assert!(!e.process(&at("KeyC", true, 60)).suppress);

        let mut s = Engine::new(&[RemapRule::Simple { from: "CapsLock".into(), to: combo(&["Ctrl"], "Escape") }]);
        s.process(&at("CapsLock", true, 0));
        assert_eq!(s.reset(), vec![ev("Escape", false), ev("ControlLeft", false)]);
    }

    fn settings(rules: Vec<RemapRule>) -> RemapSettings {
        RemapSettings { rules, excluded_apps: vec![] }
    }
    fn simple(from: &str, to: &str) -> RemapRule {
        RemapRule::Simple { from: from.into(), to: combo(&[], to) }
    }

    #[test]
    fn valid_rules_pass() {
        let s = settings(vec![simple("CapsLock", "Escape"), simple("AltRight", "ControlRight")]);
        assert!(validate_rules(&s).is_empty());
    }

    #[test]
    fn unknown_key_and_empty_combo_are_errors() {
        let s = settings(vec![
            simple("Foo", "Escape"),
            RemapRule::Simple { from: "KeyQ".into(), to: KeyCombo { mods: vec![], key: None } },
            RemapRule::TapHold { key: "KeyW".into(), tap: combo(&["Hyper"], "KeyA"), hold: combo(&[], "Bar") },
        ]);
        let fields: Vec<_> = validate_rules(&s).into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["remap.rules.0.from", "remap.rules.1.to", "remap.rules.2.tap", "remap.rules.2.hold"]);
    }

    #[test]
    fn duplicate_triggers_mark_every_field() {
        let s = settings(vec![
            simple("CapsLock", "Escape"),
            RemapRule::Layer { trigger: "CapsLock".into(), tap: None, mappings: vec![] },
            RemapRule::TapHold { key: "CapsLock".into(), tap: combo(&[], "KeyA"), hold: combo(&["Ctrl"], "KeyA") },
        ]);
        let mut fields: Vec<_> = validate_rules(&s).into_iter().map(|e| e.field).collect();
        fields.sort();
        assert_eq!(fields, ["remap.rules.0.from", "remap.rules.1.trigger", "remap.rules.2.key"]);
    }

    #[test]
    fn layer_mapping_and_macro_errors() {
        let s = settings(vec![
            RemapRule::Layer {
                trigger: "CapsLock".into(),
                tap: None,
                mappings: vec![
                    LayerMapping { from: "KeyH".into(), to: combo(&[], "ArrowLeft") },
                    LayerMapping { from: "KeyH".into(), to: combo(&[], "ArrowRight") },
                    LayerMapping { from: "Nope".into(), to: combo(&[], "ArrowUp") },
                ],
            },
            RemapRule::Macro { trigger: "".into(), steps: vec![] },
            RemapRule::Macro { trigger: "Ctrl+KeyM".into(), steps: vec![combo(&[], "Zzz")] },
        ]);
        let fields: Vec<_> = validate_rules(&s).into_iter().map(|e| e.field).collect();
        assert_eq!(
            fields,
            [
                "remap.rules.0.mappings.0.from", "remap.rules.0.mappings.1.from", "remap.rules.0.mappings.2.from",
                "remap.rules.1.trigger", "remap.rules.1.steps", "remap.rules.2.steps.0"
            ]
        );
    }
}
