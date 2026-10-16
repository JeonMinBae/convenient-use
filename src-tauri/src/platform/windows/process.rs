use anyhow::{Result, bail};
use std::ffi::c_void;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetForegroundWindow, GetWindow, GetWindowTextLengthW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_MINIMIZE, SW_RESTORE,
    SetForegroundWindow, ShowWindow,
};
use windows::core::{BOOL, PWSTR};

fn hw(h: isize) -> HWND {
    HWND(h as *mut c_void)
}

/// 보이는 최상위 창을 Z-order(앞→뒤) 순으로 (hwnd, exe 경로) 로 돌려준다.
/// 소유자가 있는 창·제목 없는 창·cloaked(다른 가상 데스크톱/중단된 UWP) 창은 제외한다.
pub fn top_windows() -> Result<Vec<(isize, String)>> {
    unsafe extern "system" fn cb(h: HWND, lp: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lp.0 as *mut Vec<(isize, String)>) };
        unsafe {
            if !IsWindowVisible(h).as_bool()
                || GetWindow(h, GW_OWNER).is_ok_and(|o| !o.0.is_null())
                || GetWindowTextLengthW(h) == 0
            {
                return BOOL(1);
            }
            let mut cloaked = 0u32;
            let _ = DwmGetWindowAttribute(
                h,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut c_void,
                size_of::<u32>() as u32,
            );
            if cloaked != 0 {
                return BOOL(1);
            }
        }
        if let Some(exe) = exe_of(h) {
            out.push((h.0 as isize, exe));
        }
        BOOL(1)
    }
    let mut out: Vec<(isize, String)> = Vec::new();
    unsafe { EnumWindows(Some(cb), LPARAM(&mut out as *mut _ as isize))? };
    Ok(out)
}

fn exe_of(h: HWND) -> Option<String> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(proc, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(proc);
        r.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// 포그라운드 창의 실행 파일 이름 (소문자, 예: "notepad.exe")
pub fn foreground_exe() -> Result<String> {
    let h = unsafe { GetForegroundWindow() };
    if h.0.is_null() {
        bail!("포그라운드 창이 없다");
    }
    let path = exe_of(h).ok_or_else(|| anyhow::anyhow!("실행 파일을 알 수 없다"))?;
    Ok(path.rsplit(['\\', '/']).next().unwrap_or(&path).to_lowercase())
}

/// 최소화 상태면 복원 후 전면으로. 포그라운드 잠금으로 실패하면 포그라운드 스레드에 붙어 재시도한다.
pub fn focus(hwnd: isize) -> Result<()> {
    let h = hw(hwnd);
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        if SetForegroundWindow(h).as_bool() {
            return Ok(());
        }
        let fg_thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let ok = SetForegroundWindow(h).as_bool();
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
        if !ok {
            bail!("SetForegroundWindow 실패");
        }
    }
    Ok(())
}

pub fn minimize(hwnd: isize) -> Result<()> {
    // ShowWindow 반환값은 이전 표시 상태라 성공 여부로 쓸 수 없다
    let _ = unsafe { ShowWindow(hw(hwnd), SW_MINIMIZE) };
    Ok(())
}
