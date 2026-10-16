use anyhow::{Result, bail};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    GetAsyncKeyState, SendInput, VIRTUAL_KEY,
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

fn key_vk(code: &str) -> Result<u16> {
    crate::keyboard::keys::code_to_vk(code).ok_or_else(|| anyhow::anyhow!("지원하지 않는 키: {code}"))
}

/// 화살표·편집 키 등은 확장 키 플래그가 있어야 방향키로 인식된다 (없으면 숫자패드 키로 해석).
fn is_extended(vk: u16) -> bool {
    matches!(vk, 0x21..=0x28 | 0x2C | 0x2D | 0x2E | 0x5B | 0x5C | 0x5D | 0x6F | 0x90 | 0xA3 | 0xA5)
}

pub fn send_keys(combos: &[KeyCombo]) -> Result<()> {
    let mut seq = Vec::new();
    for combo in combos {
        seq.extend(sequence(combo)?);
    }
    send_vks(&seq)
}

/// (W3C code, 누름 여부) 를 그대로 주입한다. 리매핑 엔진 출력용.
pub fn send_events(events: &[(String, bool)]) -> Result<()> {
    let seq = events.iter().map(|(c, down)| key_vk(c).map(|vk| (vk, *down))).collect::<Result<Vec<_>>>()?;
    send_vks(&seq)
}

fn send_vks(seq: &[(u16, bool)]) -> Result<()> {
    let inputs: Vec<INPUT> = seq
        .iter()
        .map(|&(vk, down)| {
            let mut flags = KEYBD_EVENT_FLAGS(0);
            if !down {
                flags |= KEYEVENTF_KEYUP;
            }
            if is_extended(vk) {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: INJECTED_MARK },
                },
            }
        })
        .collect();
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        bail!("SendInput 실패: {sent}/{} 전송", inputs.len());
    }
    Ok(())
}

/// 물리적으로 눌려 있는 Shift/Ctrl/Alt/Win 의 W3C code. 좌우를 구분한다.
pub fn held_modifiers() -> Vec<String> {
    const MODS: [(u16, &str); 8] = [
        (0xA0, "ShiftLeft"),
        (0xA1, "ShiftRight"),
        (0xA2, "ControlLeft"),
        (0xA3, "ControlRight"),
        (0xA4, "AltLeft"),
        (0xA5, "AltRight"),
        (0x5B, "MetaLeft"),
        (0x5C, "MetaRight"),
    ];
    // 최상위 비트(0x8000)가 현재 눌림 상태. i16 이므로 음수가 눌림이다
    MODS.iter()
        .filter(|(vk, _)| unsafe { GetAsyncKeyState(*vk as i32) } < 0)
        .map(|(_, code)| code.to_string())
        .collect()
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
