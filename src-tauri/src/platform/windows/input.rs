use anyhow::{Result, bail};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    SendInput, VIRTUAL_KEY,
};

use crate::remap::KeyCombo;

/// 주입 이벤트의 dwExtraInfo. 키 훅이 자기가 보낸 입력을 다시 가공하지 않도록 구분하는 표식이다.
pub const INJECTED_MARK: usize = 0x434F_4E56; // "CONV"

/// (가상 키, 누름 여부). 수정자는 먼저 누르고 나중에 뗀다.
fn sequence(combo: &KeyCombo) -> Result<Vec<(u16, bool)>> {
    let mut mods = Vec::new();
    for m in &combo.mods {
        mods.push(match m.as_str() {
            "Ctrl" => 0x11,
            "Alt" => 0x12,
            "Shift" => 0x10,
            "Super" => 0x5B,
            other => bail!("알 수 없는 수정자: {other}"),
        });
    }
    let mut seq: Vec<(u16, bool)> = mods.iter().map(|&vk| (vk, true)).collect();
    if let Some(key) = &combo.key {
        let vk = key_vk(key)?;
        seq.push((vk, true));
        seq.push((vk, false));
    }
    seq.extend(mods.iter().rev().map(|&vk| (vk, false)));
    Ok(seq)
}

// Task 9 의 keyboard/keys.rs 표로 대체될 최소 매핑
fn key_vk(code: &str) -> Result<u16> {
    let vk = match code {
        "Backspace" => 0x08,
        "Tab" => 0x09,
        "Enter" => 0x0D,
        "Escape" => 0x1B,
        "ArrowLeft" => 0x25,
        "ArrowUp" => 0x26,
        "ArrowRight" => 0x27,
        "ArrowDown" => 0x28,
        _ => {
            if let Some(c) = code.strip_prefix("Key").and_then(single_char).filter(char::is_ascii_uppercase) {
                c as u16
            } else if let Some(c) = code.strip_prefix("Digit").and_then(single_char).filter(char::is_ascii_digit) {
                c as u16
            } else {
                bail!("지원하지 않는 키: {code}");
            }
        }
    };
    Ok(vk)
}

fn single_char(s: &str) -> Option<char> {
    let mut it = s.chars();
    let c = it.next()?;
    it.next().is_none().then_some(c)
}

/// 화살표 등은 확장 키 플래그가 있어야 방향키로 인식된다 (없으면 숫자패드 키로 해석).
fn is_extended(vk: u16) -> bool {
    matches!(vk, 0x25..=0x28 | 0x5B)
}

pub fn send_keys(combos: &[KeyCombo]) -> Result<()> {
    let mut inputs = Vec::new();
    for combo in combos {
        for (vk, down) in sequence(combo)? {
            let mut flags = KEYBD_EVENT_FLAGS(0);
            if !down {
                flags |= KEYEVENTF_KEYUP;
            }
            if is_extended(vk) {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: INJECTED_MARK },
                },
            });
        }
    }
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        bail!("SendInput 실패: {sent}/{} 전송", inputs.len());
    }
    Ok(())
}

pub fn send_paste() -> Result<()> {
    send_keys(&[KeyCombo { mods: vec!["Ctrl".into()], key: Some("KeyV".into()) }])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(mods: &[&str], key: Option<&str>) -> KeyCombo {
        KeyCombo { mods: mods.iter().map(|s| s.to_string()).collect(), key: key.map(String::from) }
    }

    #[test]
    fn modifiers_wrap_key_in_reverse_release_order() {
        let seq = sequence(&combo(&["Ctrl", "Shift"], Some("KeyV"))).unwrap();
        assert_eq!(
            seq,
            vec![(0x11, true), (0x10, true), (0x56, true), (0x56, false), (0x10, false), (0x11, false)]
        );
    }

    #[test]
    fn modifier_only_and_key_mapping() {
        assert_eq!(sequence(&combo(&["Alt"], None)).unwrap(), vec![(0x12, true), (0x12, false)]);
        assert_eq!(key_vk("Digit7").unwrap(), 0x37);
        assert_eq!(key_vk("ArrowDown").unwrap(), 0x28);
        assert!(key_vk("KeyAB").is_err());
        assert!(key_vk("Foo").is_err());
        assert!(sequence(&combo(&["Hyper"], None)).is_err());
    }
}
