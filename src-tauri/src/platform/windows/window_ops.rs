use crate::window_layout::Rect;
use anyhow::{Result, bail};
use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromPoint, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetWindowRect, IsZoomed, SW_RESTORE, SWP_NOACTIVATE, SWP_NOZORDER,
    SetWindowPos, ShowWindow,
};
use windows::core::BOOL;

// HWND 는 포인터 타입이라 경계 밖에는 isize 로 넘긴다.
fn hw(h: isize) -> HWND {
    HWND(h as *mut c_void)
}

fn to_rect(r: RECT) -> Rect {
    Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
}

pub fn foreground() -> Result<isize> {
    let h = unsafe { GetForegroundWindow() };
    if h.0.is_null() {
        bail!("포그라운드 창이 없다");
    }
    Ok(h.0 as isize)
}

/// 사용자에게 보이는 프레임 (DWM 확장 프레임 경계).
pub fn frame(hwnd: isize) -> Result<Rect> {
    let mut r = RECT::default();
    unsafe {
        DwmGetWindowAttribute(
            hw(hwnd),
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut RECT as *mut c_void,
            size_of::<RECT>() as u32,
        )?;
    }
    Ok(to_rect(r))
}

pub fn monitor_work(hwnd: isize) -> Result<Rect> {
    let m = unsafe { MonitorFromWindow(hw(hwnd), MONITOR_DEFAULTTONEAREST) };
    work_of(m)
}

/// 마우스 커서가 있는 모니터의 작업 영역.
pub fn monitor_at_cursor() -> Result<Rect> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p)? };
    work_of(unsafe { MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST) })
}

fn work_of(m: HMONITOR) -> Result<Rect> {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    if !unsafe { GetMonitorInfoW(m, &mut info) }.as_bool() {
        bail!("GetMonitorInfoW 실패");
    }
    Ok(to_rect(info.rcWork))
}

pub fn monitors() -> Result<Vec<Rect>> {
    unsafe extern "system" fn cb(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lp.0 as *mut Vec<Rect>) };
        if let Ok(r) = work_of(m) {
            out.push(r);
        }
        BOOL(1)
    }
    let mut out: Vec<Rect> = Vec::new();
    let ok = unsafe { EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut out as *mut _ as isize)) };
    if !ok.as_bool() {
        bail!("EnumDisplayMonitors 실패");
    }
    Ok(out)
}

/// target 은 보이는 프레임 기준. 보이지 않는 리사이즈 테두리 크기만큼 SetWindowPos 사각형을 넓힌다.
pub fn set_frame(hwnd: isize, target: Rect) -> Result<()> {
    let h = hw(hwnd);
    unsafe {
        // 최대화 상태에서는 SetWindowPos 가 무시되므로 먼저 복원
        if IsZoomed(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let mut outer = RECT::default();
        GetWindowRect(h, &mut outer)?;
        let vis = frame(hwnd)?;
        let (l, t) = (vis.x - outer.left, vis.y - outer.top);
        let (r, b) = (outer.right - (vis.x + vis.w), outer.bottom - (vis.y + vis.h));
        SetWindowPos(
            h,
            None,
            target.x - l,
            target.y - t,
            target.w + l + r,
            target.h + t + b,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}
