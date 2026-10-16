use thiserror::Error;

const MOD_ORDER: [&str; 4] = ["Ctrl", "Alt", "Shift", "Super"];

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AccelError {
    #[error("키가 없습니다")]
    NoKey,
    #[error("키는 하나만 지정할 수 있습니다")]
    MultipleKeys,
    #[error("Ctrl/Alt/Shift/Super 중 하나 이상이 필요합니다")]
    NoModifier,
    #[error("지원하지 않는 키입니다: {0}")]
    UnknownKey(String),
}

/// 표기만 다른 단축키를 같은 문자열로 맞춘다. 결과는 global-hotkey 파서가 그대로 받아들이는 형식이다.
pub fn normalize(input: &str) -> Result<String, AccelError> {
    let mut mods = [false; 4];
    let mut key: Option<String> = None;
    for raw in input.split('+').map(str::trim).filter(|s| !s.is_empty()) {
        let m = match raw.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(0),
            "alt" | "option" => Some(1),
            "shift" => Some(2),
            "super" | "win" | "cmd" | "command" | "meta" => Some(3),
            _ => None,
        };
        match m {
            Some(i) => mods[i] = true,
            None if key.is_none() => key = Some(canonical_code(raw)?),
            None => return Err(AccelError::MultipleKeys),
        }
    }
    let key = key.ok_or(AccelError::NoKey)?;
    // F키만 모디파이어 없이 전역 등록을 허용한다 (일반 키 단독은 타이핑을 가로챈다)
    if !mods.iter().any(|m| *m) && !is_function_key(&key) {
        return Err(AccelError::NoModifier);
    }
    let mut parts: Vec<&str> =
        MOD_ORDER.iter().zip(mods).filter(|(_, on)| *on).map(|(n, _)| *n).collect();
    parts.push(&key);
    Ok(parts.join("+"))
}

fn is_function_key(key: &str) -> bool {
    key.strip_prefix('F').is_some_and(|n| n.parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)))
}

// 플러그인(global-hotkey)이 받아들이는 Code 이름. 이 표에 있는 이름만 통과시킨다.
const CODES: &[&str] = &[
    "Backquote", "Backslash", "BracketLeft", "BracketRight", "Comma", "Equal", "Minus", "Period",
    "Quote", "Semicolon", "Slash", "Backspace", "Enter", "Space", "Tab", "Delete", "End", "Home",
    "Insert", "PageDown", "PageUp", "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowUp", "Escape",
    "PrintScreen", "Pause", "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5",
    "Numpad6", "Numpad7", "Numpad8", "Numpad9", "NumpadAdd", "NumpadDecimal", "NumpadDivide",
    "NumpadMultiply", "NumpadSubtract",
];

// 별칭은 소문자로 비교한다. 이름이 겹치지 않도록 CODES 와 별도로 둔다.
const ALIASES: &[(&str, &str)] = &[
    ("`", "Backquote"), ("\\", "Backslash"), ("[", "BracketLeft"), ("]", "BracketRight"),
    (",", "Comma"), ("=", "Equal"), ("-", "Minus"), (".", "Period"), ("'", "Quote"),
    (";", "Semicolon"), ("/", "Slash"), ("return", "Enter"), ("esc", "Escape"), ("del", "Delete"),
    ("ins", "Insert"), ("pgup", "PageUp"), ("pgdn", "PageDown"), ("left", "ArrowLeft"),
    ("right", "ArrowRight"), ("up", "ArrowUp"), ("down", "ArrowDown"),
];

/// "u" → "KeyU", "1" → "Digit1", "left" → "ArrowLeft", "return" → "Enter", 이미 Code 이름이면 대소문자만 정리.
fn canonical_code(raw: &str) -> Result<String, AccelError> {
    let lower = raw.to_ascii_lowercase();
    let unknown = || AccelError::UnknownKey(raw.to_string());

    let letter = lower.strip_prefix("key").unwrap_or(&lower);
    if letter.len() == 1 && letter.as_bytes()[0].is_ascii_lowercase() {
        return Ok(format!("Key{}", letter.to_ascii_uppercase()));
    }
    let digit = lower.strip_prefix("digit").unwrap_or(&lower);
    if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() {
        return Ok(format!("Digit{digit}"));
    }
    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok())
        && (1..=24).contains(&n)
        && lower[1..] == n.to_string()
    {
        return Ok(format!("F{n}"));
    }
    if let Some((_, code)) = ALIASES.iter().find(|(a, _)| *a == lower) {
        return Ok((*code).to_string());
    }
    CODES.iter().find(|c| c.eq_ignore_ascii_case(&lower)).map(|c| (*c).to_string()).ok_or_else(unknown)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tauri_plugin_global_shortcut::Shortcut;

    use super::*;

    #[test]
    fn normalize_orders_and_aliases() {
        assert_eq!(normalize("shift+ctrl+v").unwrap(), "Ctrl+Shift+KeyV");
        assert_eq!(normalize("Win + Alt + Control + Right").unwrap(), "Ctrl+Alt+Super+ArrowRight");
        assert_eq!(normalize("F5").unwrap(), "F5");
        assert_eq!(normalize("ctrl+1").unwrap(), "Ctrl+Digit1");
        assert_eq!(normalize("ctrl+return").unwrap(), "Ctrl+Enter");
    }

    #[test]
    fn normalize_rejects_bad_input() {
        assert_eq!(normalize("A"), Err(AccelError::NoModifier));
        assert_eq!(normalize("Ctrl+A+B"), Err(AccelError::MultipleKeys));
        assert_eq!(normalize("Ctrl+Shift"), Err(AccelError::NoKey));
        assert_eq!(normalize("Ctrl+Foo"), Err(AccelError::UnknownKey("Foo".into())));
        assert_eq!(normalize("Ctrl+F25"), Err(AccelError::UnknownKey("F25".into())));
    }

    // 정규화 결과가 플러그인의 Shortcut 파서로 그대로 변환되는지 (별도 변환기 불필요함을 보증)
    #[test]
    fn canonical_form_parses_as_plugin_shortcut() {
        let mut keys: Vec<String> = CODES.iter().map(|c| format!("Ctrl+{c}")).collect();
        keys.extend(('A'..='Z').map(|c| format!("Ctrl+Key{c}")));
        keys.extend((0..=9).map(|d| format!("Ctrl+Digit{d}")));
        keys.extend((1..=24).map(|n| format!("F{n}")));
        keys.push(normalize("Win+Alt+Control+Shift+Right").unwrap());
        for k in keys {
            assert_eq!(normalize(&k).unwrap(), k);
            Shortcut::from_str(&k).unwrap_or_else(|e| panic!("{k}: {e}"));
        }
        assert_eq!(
            Shortcut::from_str("Ctrl+Alt+Super+ArrowRight").unwrap().id(),
            Shortcut::from_str(&normalize("win+control+option+right").unwrap()).unwrap().id()
        );
    }
}
