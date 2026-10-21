use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::time::Duration;

use anyhow::{Result, bail};

use super::ffi::*;
use super::input::{self, INJECTED_MARK};
use super::keycodes::kc_to_code;
use crate::keyboard::KeyEvent;

/// true 를 돌려주면 해당 키 입력을 삼킨다.
type Callback = fn(&KeyEvent) -> bool;
static CALLBACK: OnceLock<Callback> = OnceLock::new();
static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

const KC_CAPS_LOCK: u16 = 0x39;
const KC_F18: u16 = 0x4F;
static CAPS_CAPTURE: AtomicBool = AtomicBool::new(false);

/// CapsLock 은 이벤트 탭에 누름·뗌으로 오지 않으므로(토글 때 한 번만 온다), 리매핑 규칙이 CapsLock 을 쓸 때만
/// HID 단계에서 F18 로 바꿔 받고 훅이 F18 을 "CapsLock" 으로 보고한다. 앱 종료 시 반드시 false 로 되돌린다.
/// hidutil 의 전역 UserKeyMapping 을 통째로 쓰므로 사용자가 hidutil 로 직접 건 매핑과는 함께 쓸 수 없다.
pub fn capture_caps_lock(on: bool) {
    static SYNCED: AtomicBool = AtomicBool::new(false);
    let changed = CAPS_CAPTURE.swap(on, Ordering::AcqRel) != on;
    // 첫 호출에서는 비정상 종료로 남은 우리 매핑만 정리한다 (사용자가 직접 건 매핑은 건드리지 않는다)
    let first = !SYNCED.swap(true, Ordering::AcqRel);
    if !changed && !(first && !on && our_mapping_left()) {
        return;
    }
    // HID 사용 코드: 0x700000039(CapsLock) → 0x70000006D(F18). hidutil 이 10진수 JSON 을 받는다
    let mapping = if on {
        r#"{"UserKeyMapping":[{"HIDKeyboardModifierMappingSrc":30064771129,"HIDKeyboardModifierMappingDst":30064771181}]}"#
    } else {
        r#"{"UserKeyMapping":[]}"#
    };
    match std::process::Command::new("/usr/bin/hidutil").args(["property", "--set", mapping]).output() {
        Ok(o) if o.status.success() => {}
        Ok(o) => log::warn!("hidutil 실패: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => log::warn!("hidutil 실행 실패: {e}"),
    }
}

fn our_mapping_left() -> bool {
    std::process::Command::new("/usr/bin/hidutil")
        .args(["property", "--get", "UserKeyMapping"])
        .output()
        .is_ok_and(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains("30064771129") && s.contains("30064771181")
        })
}

/// 전용 스레드에서 CGEventTap 을 설치하고 런루프를 돈다.
/// 손쉬운 사용 권한이 없으면 안내 대화상자를 띄우고, 허용될 때까지 기다렸다가 설치한다.
pub fn install(cb: Callback) -> Result<()> {
    if CALLBACK.set(cb).is_err() {
        bail!("키보드 훅이 이미 설치되어 있다");
    }
    if !ax_trusted(true) {
        log::warn!("손쉬운 사용 권한이 없어 키 리매핑·스니펫·창 배치가 동작하지 않는다. 허용하면 자동으로 켜진다");
    }
    std::thread::Builder::new().name("keyboard-hook".into()).spawn(|| {
        loop {
            if ax_trusted(false) {
                match create_tap() {
                    Some(tap) => return run(tap),
                    None => log::warn!("CGEventTapCreate 실패. 잠시 후 다시 시도한다"),
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    })?;
    Ok(())
}

fn create_tap() -> Option<CFMachPortRef> {
    let mask = (1u64 << kCGEventKeyDown) | (1u64 << kCGEventKeyUp) | (1u64 << kCGEventFlagsChanged);
    let tap = unsafe {
        CGEventTapCreate(
            kCGSessionEventTap,
            kCGHeadInsertEventTap,
            kCGEventTapOptionDefault,
            mask,
            tap_proc,
            std::ptr::null_mut(),
        )
    };
    (!tap.is_null()).then_some(tap)
}

fn run(tap: CFMachPortRef) {
    TAP.store(tap, Ordering::Release);
    unsafe {
        let src = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        CFRunLoopAddSource(CFRunLoopGetCurrent(), src, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        // 탭은 이 스레드의 런루프가 돌아야 콜백이 호출된다
        CFRunLoopRun();
    }
}

unsafe extern "C" fn tap_proc(_proxy: *mut c_void, ty: u32, event: CGEventRef, _user: *mut c_void) -> CGEventRef {
    // 콜백이 오래 걸리거나 사용자 입력으로 꺼지면 OS 가 탭을 끈다. 바로 다시 켠다
    if ty == kCGEventTapDisabledByTimeout || ty == kCGEventTapDisabledByUserInput {
        let tap = TAP.load(Ordering::Acquire);
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, true) };
        }
        return event;
    }
    // 패닉이 FFI 경계를 넘지 않도록 막는다
    let suppress = std::panic::catch_unwind(|| handle(ty, event)).unwrap_or(false);
    if suppress { std::ptr::null_mut() } else { event }
}

#[allow(non_upper_case_globals)]
fn handle(ty: u32, event: CGEventRef) -> bool {
    let Some(cb) = CALLBACK.get() else { return false };
    let kc = unsafe { CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) } as u16;
    let injected = unsafe { CGEventGetIntegerValueField(event, kCGEventSourceUserData) } == INJECTED_MARK as i64;
    let code = if kc == KC_F18 && CAPS_CAPTURE.load(Ordering::Relaxed) {
        "CapsLock"
    } else {
        kc_to_code(kc).unwrap_or("Unknown")
    };
    let flags = unsafe { CGEventGetFlags(event) };
    let down = match ty {
        kCGEventKeyDown => true,
        kCGEventKeyUp => false,
        // 실제 CapsLock 은 토글 때 한 번만 와서 누름·뗌으로 쓸 수 없다 (capture_caps_lock 참고). Fn 등도 그대로 통과시킨다
        kCGEventFlagsChanged if kc == KC_CAPS_LOCK => return false,
        // 공통 비트(0x00FF_0000)를 빼고 좌우 장치 비트로 판단해야 반대쪽 키가 눌려 있어도 정확하다
        kCGEventFlagsChanged => match input::mod_bits(code) {
            Some(b) => flags & b & !0x00FF_0000 != 0,
            None => return false,
        },
        _ => return false,
    };
    let ch = if down && ty == kCGEventKeyDown { translate_char(event) } else { None };
    let ev = KeyEvent { code: code.to_string(), down, time_ms: unsafe { CGEventGetTimestamp(event) } / 1_000_000, injected, ch };
    let suppress = cb(&ev);
    if !injected {
        if ty == kCGEventFlagsChanged {
            input::track_physical_modifier(code, down, suppress);
        }
        if !suppress && input::overrides_active() {
            unsafe { CGEventSetFlags(event, input::effective_flags(flags)) };
        }
    }
    suppress
}

/// 현재 입력 소스·수정자 상태로 이 키가 만드는 문자.
fn translate_char(event: CGEventRef) -> Option<char> {
    let mut buf = [0u16; 4];
    let mut len = 0usize;
    unsafe { CGEventKeyboardGetUnicodeString(event, buf.len(), &mut len, buf.as_mut_ptr()) };
    if len == 1 { char::from_u32(buf[0] as u32) } else { None }
}
