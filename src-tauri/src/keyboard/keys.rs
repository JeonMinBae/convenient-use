// W3C KeyboardEvent.code 와 Windows 가상 키 코드의 대응표. 좌우 수정자는 LL 훅이 주는 VK_L*/VK_R* 로 구분한다.
// NumpadEnter 는 VK_RETURN 과 같아(확장 키 플래그로만 구분) 표에 두지 않는다.

const TABLE: &[(&str, u16)] = &[
    ("KeyA", 0x41),
    ("KeyB", 0x42),
    ("KeyC", 0x43),
    ("KeyD", 0x44),
    ("KeyE", 0x45),
    ("KeyF", 0x46),
    ("KeyG", 0x47),
    ("KeyH", 0x48),
    ("KeyI", 0x49),
    ("KeyJ", 0x4A),
    ("KeyK", 0x4B),
    ("KeyL", 0x4C),
    ("KeyM", 0x4D),
    ("KeyN", 0x4E),
    ("KeyO", 0x4F),
    ("KeyP", 0x50),
    ("KeyQ", 0x51),
    ("KeyR", 0x52),
    ("KeyS", 0x53),
    ("KeyT", 0x54),
    ("KeyU", 0x55),
    ("KeyV", 0x56),
    ("KeyW", 0x57),
    ("KeyX", 0x58),
    ("KeyY", 0x59),
    ("KeyZ", 0x5A),
    ("Digit0", 0x30),
    ("Digit1", 0x31),
    ("Digit2", 0x32),
    ("Digit3", 0x33),
    ("Digit4", 0x34),
    ("Digit5", 0x35),
    ("Digit6", 0x36),
    ("Digit7", 0x37),
    ("Digit8", 0x38),
    ("Digit9", 0x39),
    ("F1", 0x70),
    ("F2", 0x71),
    ("F3", 0x72),
    ("F4", 0x73),
    ("F5", 0x74),
    ("F6", 0x75),
    ("F7", 0x76),
    ("F8", 0x77),
    ("F9", 0x78),
    ("F10", 0x79),
    ("F11", 0x7A),
    ("F12", 0x7B),
    ("F13", 0x7C),
    ("F14", 0x7D),
    ("F15", 0x7E),
    ("F16", 0x7F),
    ("F17", 0x80),
    ("F18", 0x81),
    ("F19", 0x82),
    ("F20", 0x83),
    ("F21", 0x84),
    ("F22", 0x85),
    ("F23", 0x86),
    ("F24", 0x87),
    ("Numpad0", 0x60),
    ("Numpad1", 0x61),
    ("Numpad2", 0x62),
    ("Numpad3", 0x63),
    ("Numpad4", 0x64),
    ("Numpad5", 0x65),
    ("Numpad6", 0x66),
    ("Numpad7", 0x67),
    ("Numpad8", 0x68),
    ("Numpad9", 0x69),
    ("NumpadMultiply", 0x6A),
    ("NumpadAdd", 0x6B),
    ("NumpadSubtract", 0x6D),
    ("NumpadDecimal", 0x6E),
    ("NumpadDivide", 0x6F),
    ("Backspace", 0x08),
    ("Tab", 0x09),
    ("Enter", 0x0D),
    ("Pause", 0x13),
    ("CapsLock", 0x14),
    ("Lang1", 0x15),
    ("Lang2", 0x19),
    ("Escape", 0x1B),
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("ArrowLeft", 0x25),
    ("ArrowUp", 0x26),
    ("ArrowRight", 0x27),
    ("ArrowDown", 0x28),
    ("PrintScreen", 0x2C),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("MetaLeft", 0x5B),
    ("MetaRight", 0x5C),
    ("ContextMenu", 0x5D),
    ("NumLock", 0x90),
    ("ScrollLock", 0x91),
    ("ShiftLeft", 0xA0),
    ("ShiftRight", 0xA1),
    ("ControlLeft", 0xA2),
    ("ControlRight", 0xA3),
    ("AltLeft", 0xA4),
    ("AltRight", 0xA5),
    ("Semicolon", 0xBA),
    ("Equal", 0xBB),
    ("Comma", 0xBC),
    ("Minus", 0xBD),
    ("Period", 0xBE),
    ("Slash", 0xBF),
    ("Backquote", 0xC0),
    ("BracketLeft", 0xDB),
    ("Backslash", 0xDC),
    ("BracketRight", 0xDD),
    ("Quote", 0xDE),
];

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn code_to_vk(code: &str) -> Option<u16> {
    TABLE.iter().find(|(c, _)| *c == code).map(|&(_, vk)| vk)
}

// Windows 훅만 쓴다 (macOS 는 platform::macos::keycodes 의 자체 표)
#[cfg_attr(not(windows), allow(dead_code))]
pub fn vk_to_code(vk: u16) -> Option<&'static str> {
    TABLE.iter().find(|&&(_, v)| v == vk).map(|&(c, _)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_code_roundtrip() {
        for code in ["KeyA", "Digit0", "ArrowLeft", "CapsLock", "Escape", "Enter", "F12", "Semicolon", "ShiftLeft"] {
            assert_eq!(vk_to_code(code_to_vk(code).unwrap()), Some(code));
        }
    }

    #[test]
    fn whole_table_roundtrips_without_duplicates() {
        for &(code, vk) in TABLE {
            assert_eq!(code_to_vk(code), Some(vk));
            assert_eq!(vk_to_code(vk), Some(code), "VK 중복: {vk:#x}");
        }
    }

    #[test]
    fn ime_keys_and_unknown() {
        assert_eq!(code_to_vk("Lang1"), Some(0x15));
        assert_eq!(code_to_vk("Lang2"), Some(0x19));
        assert_eq!(code_to_vk("Nope"), None);
        assert_eq!(vk_to_code(0xE5), None);
    }
}
