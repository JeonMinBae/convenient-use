use std::sync::OnceLock;
use std::sync::mpsc;

use anyhow::{Result, anyhow, bail};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, VK_CAPITAL, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, KBDLLHOOKSTRUCT,
    MSG, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

use super::input::INJECTED_MARK;
use crate::keyboard::{KeyEvent, keys};

/// true 를 돌려주면 해당 키 입력을 삼킨다.
type Callback = fn(&KeyEvent) -> bool;
static CALLBACK: OnceLock<Callback> = OnceLock::new();

const VK_PROCESSKEY: u32 = 0xE5;

/// 전용 스레드에서 WH_KEYBOARD_LL 훅을 설치하고 메시지 루프를 돈다. 설치 결과를 기다려 돌려준다.
pub fn install(cb: Callback) -> Result<()> {
    if CALLBACK.set(cb).is_err() {
        bail!("키보드 훅이 이미 설치되어 있다");
    }
    let (tx, rx) = mpsc::channel::<Result<()>>();
    std::thread::Builder::new().name("keyboard-hook".into()).spawn(move || unsafe {
        let hook = GetModuleHandleW(None)
            .map_err(|e| anyhow!("GetModuleHandleW 실패: {e}"))
            .and_then(|m| {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(HINSTANCE(m.0)), 0)
                    .map_err(|e| anyhow!("SetWindowsHookExW 실패: {e}"))
            });
        let hook = match hook {
            Ok(h) => {
                let _ = tx.send(Ok(()));
                h
            }
            Err(e) => {
                let _ = tx.send(Err(e));
                return;
            }
        };
        // LL 훅은 설치한 스레드의 메시지 루프가 돌아야 콜백이 호출된다
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(hook);
    })?;
    rx.recv().map_err(|_| anyhow!("훅 스레드가 응답 없이 종료됨"))?
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        // 패닉이 FFI 경계를 넘지 않도록 막는다
        let suppress = std::panic::catch_unwind(|| match CALLBACK.get() {
            Some(cb) => cb(&to_event(info, down)),
            None => false,
        })
        .unwrap_or(false);
        if suppress {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn to_event(info: &KBDLLHOOKSTRUCT, down: bool) -> KeyEvent {
    let vk = info.vkCode;
    let (code, ch) = if vk == VK_PROCESSKEY {
        ("Process".to_string(), None)
    } else {
        let code = u16::try_from(vk).ok().and_then(keys::vk_to_code).unwrap_or("Unknown").to_string();
        (code, if down { translate_char(vk, info.scanCode) } else { None })
    };
    KeyEvent { code, down, time_ms: info.time as u64, injected: info.dwExtraInfo == INJECTED_MARK, ch }
}

/// wFlags 0x4: 키보드 상태(데드키 버퍼)를 바꾸지 않는다. 포그라운드 창의 레이아웃으로 변환한다.
fn translate_char(vk: u32, scan: u32) -> Option<char> {
    let mut state = [0u8; 256];
    unsafe {
        if GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 {
            state[VK_SHIFT.0 as usize] = 0x80;
        }
        if GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 {
            state[VK_CAPITAL.0 as usize] = 1;
        }
        let tid = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let hkl = GetKeyboardLayout(tid);
        let mut buf = [0u16; 8];
        let n = ToUnicodeEx(vk, scan, &state, &mut buf, 0x4, Some(hkl));
        if n == 1 { char::from_u32(buf[0] as u32) } else { None }
    }
}
