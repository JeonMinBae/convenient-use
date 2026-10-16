use serde::Serialize;

use super::model::Settings;
use crate::hotkey::registry::{collect, find_conflicts};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// 저장 전 검증. 형식 오류와 단축키 충돌(충돌한 각 필드마다 하나씩)을 모두 돌려준다.
pub fn validate(s: &Settings) -> Vec<ValidationError> {
    let (bindings, invalid) = collect(s);
    let mut errors: Vec<ValidationError> = invalid
        .into_iter()
        .map(|b| ValidationError {
            field: b.source.field(),
            message: format!("단축키 형식이 올바르지 않습니다 ({}): {}", b.raw, b.error),
        })
        .collect();
    for group in find_conflicts(&bindings) {
        for source in &group {
            let others: Vec<String> =
                group.iter().filter(|o| *o != source).map(|o| o.field()).collect();
            errors.push(ValidationError {
                field: source.field(),
                message: format!("다른 단축키와 충돌합니다: {}", others.join(", ")),
            });
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::{LaunchItem, LaunchTarget, OsPath};

    fn item(id: &str, hotkey: &str) -> LaunchItem {
        LaunchItem {
            id: id.into(),
            name: id.into(),
            hotkey: hotkey.into(),
            target: LaunchTarget::Url { url: "https://x".into(), browser: OsPath::default() },
        }
    }

    #[test]
    fn defaults_are_valid() {
        assert!(validate(&Settings::default()).is_empty());
    }

    #[test]
    fn conflict_marks_every_field() {
        let mut s = Settings::default();
        s.launch_items.push(item("a", "Alt+Ctrl+Left"));
        let fields: Vec<String> = validate(&s).into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["launchItems.a.hotkey", "windowHotkeys.leftHalf"]);
    }

    #[test]
    fn invalid_accelerator_is_error() {
        let mut s = Settings::default();
        s.clipboard.hotkey = "V".into();
        let errs = validate(&s);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].field, "clipboard.hotkey");
    }
}
