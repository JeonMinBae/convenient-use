use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow, bail};

use super::ffi::*;
use super::keycodes::code_to_kc;
use crate::remap::KeyCombo;

/// 주입 이벤트의 kCGEventSourceUserData. 키 훅이 자기가 보낸 입력을 다시 가공하지 않도록 구분하는 표식이다.
pub const INJECTED_MARK: usize = 0x434F_4E56; // "CONV"

// macOS 는 Windows 와 달리 키 이벤트마다 수정자 플래그를 따로 싣고, 하드웨어 상태로 채운다.
// 그래서 리매핑이 삼킨 물리 수정자(FORCE_OFF)와 주입으로 누른 수정자(FORCE_ON)를 직접 추적해
// 주입 이벤트와 통과시키는 이벤트의 플래그를 고쳐 쓴다. 둘 다 0 이면 아무것도 바꾸지 않는다.
static FORCE_ON: AtomicU64 = AtomicU64::new(0);
static FORCE_OFF: AtomicU64 = AtomicU64::new(0);

/// 수정자 키의 (공통 플래그 | 좌우 장치 플래그). NX_DEVICE*KEYMASK 값이다.
pub(super) fn mod_bits(code: &str) -> Option<u64> {
    Some(match code {
        "ControlLeft" => kCGEventFlagMaskControl | 0x0001,
        "ControlRight" => kCGEventFlagMaskControl | 0x2000,
        "ShiftLeft" => kCGEventFlagMaskShift | 0x0002,
        "ShiftRight" => kCGEventFlagMaskShift | 0x0004,
        "MetaLeft" => kCGEventFlagMaskCommand | 0x0008,
        "MetaRight" => kCGEventFlagMaskCommand | 0x0010,
        "AltLeft" => kCGEventFlagMaskAlternate | 0x0020,
        "AltRight" => kCGEventFlagMaskAlternate | 0x0040,
        _ => return None,
    })
}

pub(super) fn overrides_active() -> bool {
    FORCE_ON.load(Ordering::Relaxed) | FORCE_OFF.load(Ordering::Relaxed) != 0
}

pub(super) fn effective_flags(hw: u64) -> u64 {
    (hw & !FORCE_OFF.load(Ordering::Relaxed)) | FORCE_ON.load(Ordering::Relaxed)
}

/// 훅이 물리 수정자 이벤트를 처리한 뒤 호출한다. 삼킨 누름은 떼기 전까지 플래그에서 뺀다.
pub(super) fn track_physical_modifier(code: &str, down: bool, suppressed: bool) {
    let Some(b) = mod_bits(code) else { return };
    if down && suppressed {
        FORCE_OFF.fetch_or(b, Ordering::Relaxed);
    } else if !down {
        FORCE_OFF.fetch_and(!b, Ordering::Relaxed);
    }
}

/// 화살표·편집·F 키는 하드웨어가 Fn(·숫자패드) 플래그를 붙여 보낸다. 플래그를 직접 쓰므로 같이 붙인다.
fn key_flags(kc: u16) -> u64 {
    match kc {
        0x7B..=0x7E => kCGEventFlagMaskNumericPad | kCGEventFlagMaskSecondaryFn,
        0x72..=0x79 | 0x60..=0x6F | 0x40 | 0x4F | 0x50 | 0x5A => kCGEventFlagMaskSecondaryFn,
        _ => 0,
    }
}

struct Source(CGEventSourceRef);
// CGEventSource 는 스레드 안전한 CF 객체다
unsafe impl Send for Source {}
unsafe impl Sync for Source {}

/// Private 상태 소스: 이벤트 플래그가 하드웨어 수정자와 합쳐지지 않고 우리가 정한 값 그대로 간다.
fn source() -> CGEventSourceRef {
    static S: OnceLock<Source> = OnceLock::new();
    S.get_or_init(|| Source(unsafe { CGEventSourceCreate(kCGEventSourceStatePrivate) })).0
}

fn post(code: &str, down: bool) -> Result<()> {
    let kc = code_to_kc(code).ok_or_else(|| anyhow!("지원하지 않는 키: {code}"))?;
    let modifier = mod_bits(code);
    if let Some(b) = modifier {
        if down {
            FORCE_ON.fetch_or(b, Ordering::Relaxed);
            FORCE_OFF.fetch_and(!b, Ordering::Relaxed);
        } else {
            FORCE_ON.fetch_and(!b, Ordering::Relaxed);
            // 물리적으로 누르고 있는 키를 떼는 주입이면, 사용자가 실제로 뗄 때까지 플래그에서 뺀다
            if unsafe { CGEventSourceKeyState(kCGEventSourceStateHIDSystemState, kc) } {
                FORCE_OFF.fetch_or(b, Ordering::Relaxed);
            }
        }
    }
    let ev = unsafe { CGEventCreateKeyboardEvent(source(), kc, down) };
    if ev.is_null() {
        bail!("CGEventCreateKeyboardEvent 실패: {code}");
    }
    let hw = unsafe { CGEventSourceFlagsState(kCGEventSourceStateHIDSystemState) };
    unsafe {
        if modifier.is_some() {
            CGEventSetType(ev, kCGEventFlagsChanged);
        }
        CGEventSetFlags(ev, effective_flags(hw) | key_flags(kc));
        CGEventSetIntegerValueField(ev, kCGEventSourceUserData, INJECTED_MARK as i64);
        CGEventPost(kCGHIDEventTap, ev);
        CFRelease(ev as CFTypeRef);
    }
    Ok(())
}

/// 이 OS 에서 주입·인식할 수 있는 키인지 (리매핑 규칙 검증용)
pub fn supports_key(code: &str) -> bool {
    code_to_kc(code).is_some()
}

fn mod_code(m: &str) -> Result<&'static str> {
    Ok(match m {
        "Ctrl" => "ControlLeft",
        "Alt" => "AltLeft",
        "Shift" => "ShiftLeft",
        "Super" => "MetaLeft",
        other => bail!("알 수 없는 수정자: {other}"),
    })
}

/// (code, 누름 여부). 수정자는 먼저 누르고 나중에 뗀다.
fn sequence(combo: &KeyCombo) -> Result<Vec<(String, bool)>> {
    let mods = combo.mods.iter().map(|m| mod_code(m)).collect::<Result<Vec<_>>>()?;
    let mut seq: Vec<(String, bool)> = mods.iter().map(|m| (m.to_string(), true)).collect();
    if let Some(key) = &combo.key {
        code_to_kc(key).ok_or_else(|| anyhow!("지원하지 않는 키: {key}"))?;
        seq.push((key.clone(), true));
        seq.push((key.clone(), false));
    }
    seq.extend(mods.iter().rev().map(|m| (m.to_string(), false)));
    Ok(seq)
}

pub fn send_keys(combos: &[KeyCombo]) -> Result<()> {
    let mut seq = Vec::new();
    for combo in combos {
        seq.extend(sequence(combo)?);
    }
    send_events(&seq)
}

/// (W3C code, 누름 여부) 를 그대로 주입한다. 리매핑 엔진 출력용.
pub fn send_events(events: &[(String, bool)]) -> Result<()> {
    // 일부만 보내고 실패하면 눌린 키가 남으므로 먼저 전부 검사한다
    if let Some((c, _)) = events.iter().find(|(c, _)| code_to_kc(c).is_none()) {
        bail!("지원하지 않는 키: {c}");
    }
    events.iter().try_for_each(|(c, down)| post(c, *down))
}

/// 물리적으로 눌려 있는 Shift/Control/Option/Command 의 W3C code. 좌우를 구분한다.
pub fn held_modifiers() -> Vec<String> {
    const MODS: [&str; 8] =
        ["ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"];
    MODS.iter()
        .filter(|c| {
            code_to_kc(c).is_some_and(|kc| unsafe { CGEventSourceKeyState(kCGEventSourceStateHIDSystemState, kc) })
        })
        .map(|c| c.to_string())
        .collect()
}

pub fn send_paste() -> Result<()> {
    send_keys(&[KeyCombo { mods: vec!["Super".into()], key: Some("KeyV".into()) }])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(mods: &[&str], key: Option<&str>) -> KeyCombo {
        KeyCombo { mods: mods.iter().map(|s| s.to_string()).collect(), key: key.map(String::from) }
    }

    #[test]
    fn modifiers_wrap_key_in_reverse_release_order() {
        let seq = sequence(&combo(&["Super", "Shift"], Some("KeyV"))).unwrap();
        let expect = [("MetaLeft", true), ("ShiftLeft", true), ("KeyV", true), ("KeyV", false), ("ShiftLeft", false), ("MetaLeft", false)];
        assert_eq!(seq, expect.map(|(c, d)| (c.to_string(), d)));
        assert!(sequence(&combo(&["Hyper"], None)).is_err());
        assert!(sequence(&combo(&[], Some("PrintScreen"))).is_err());
    }
}
