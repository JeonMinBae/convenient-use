// 창 핸들(isize)은 앱의 pid 다. 대상 창은 그 앱의 포커스 창(AXFocusedWindow)이다.
// 좌표는 주 화면 왼쪽 위 원점의 포인트 단위(Accessibility 좌표계)로 통일한다.
use std::ffi::c_void;

use anyhow::{Context, Result, bail};

use super::appkit;
use super::ffi::*;
use crate::window_layout::Rect;

fn pid(h: isize) -> i32 {
    h as i32
}

pub(super) fn ax_attr(el: CFTypeRef, name: &str) -> Option<Cf> {
    let mut v: CFTypeRef = std::ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, cfstr(name).0, &mut v) };
    if err == kAXErrorSuccess { Cf::new(v) } else { None }
}

/// 앱의 포커스 창. 없으면(창 전환 중 등) 메인 창.
pub(super) fn focused_window(pid: i32) -> Option<Cf> {
    let app = Cf::new(unsafe { AXUIElementCreateApplication(pid) })?;
    ax_attr(app.0, "AXFocusedWindow").or_else(|| ax_attr(app.0, "AXMainWindow"))
}

fn ax_value<T: Default>(el: CFTypeRef, name: &str, ty: u32) -> Option<T> {
    let v = ax_attr(el, name)?;
    let mut out = T::default();
    (unsafe { AXValueGetValue(v.0, ty, &mut out as *mut T as *mut c_void) } != 0).then_some(out)
}

fn set_ax_value<T>(el: CFTypeRef, name: &str, ty: u32, value: &T) -> Result<()> {
    let v = Cf::new(unsafe { AXValueCreate(ty, value as *const T as *const c_void) }).context("AXValueCreate 실패")?;
    let err = unsafe { AXUIElementSetAttributeValue(el, cfstr(name).0, v.0) };
    if err != kAXErrorSuccess {
        bail!("{name} 설정 실패 (AXError {err})");
    }
    Ok(())
}

fn to_rect(r: CGRect) -> Rect {
    Rect {
        x: r.origin.x.round() as i32,
        y: r.origin.y.round() as i32,
        w: r.size.width.round() as i32,
        h: r.size.height.round() as i32,
    }
}

/// Cocoa 좌표(왼쪽 아래 원점, y 위쪽) → 주 화면 왼쪽 위 원점 좌표.
fn flip(r: CGRect, primary_h: f64) -> Rect {
    to_rect(CGRect { origin: CGPoint { x: r.origin.x, y: primary_h - (r.origin.y + r.size.height) }, size: r.size })
}

/// (전체 프레임, 작업 영역)을 위쪽 원점 좌표로.
fn screens() -> Result<Vec<(Rect, Rect)>> {
    let raw = appkit::screens();
    let primary_h = raw.first().context("화면 정보를 얻지 못했다")?.0.size.height;
    Ok(raw.into_iter().map(|(f, v)| (flip(f, primary_h), flip(v, primary_h))).collect())
}

fn contains(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

fn work_at(x: i32, y: i32) -> Result<Rect> {
    let all = screens()?;
    Ok(all.iter().find(|(f, _)| contains(*f, x, y)).unwrap_or(&all[0]).1)
}

pub fn foreground() -> Result<isize> {
    appkit::frontmost_pid().map(|p| p as isize).context("포그라운드 앱이 없다")
}

pub fn frame(hwnd: isize) -> Result<Rect> {
    let win = focused_window(pid(hwnd)).context("포커스된 창이 없다 (손쉬운 사용 권한을 확인하세요)")?;
    let pos: CGPoint = ax_value(win.0, "AXPosition", kAXValueCGPointType).context("창 위치를 읽지 못했다")?;
    let size: CGSize = ax_value(win.0, "AXSize", kAXValueCGSizeType).context("창 크기를 읽지 못했다")?;
    Ok(to_rect(CGRect { origin: pos, size }))
}

/// 창 중심이 속한 화면의 작업 영역.
pub fn monitor_work(hwnd: isize) -> Result<Rect> {
    let f = frame(hwnd)?;
    work_at(f.x + f.w / 2, f.y + f.h / 2)
}

pub fn monitor_at_cursor() -> Result<Rect> {
    let raw = appkit::screens();
    let primary_h = raw.first().context("화면 정보를 얻지 못했다")?.0.size.height;
    let p = appkit::mouse_location();
    work_at(p.x.round() as i32, (primary_h - p.y).round() as i32)
}

pub fn monitors() -> Result<Vec<Rect>> {
    Ok(screens()?.into_iter().map(|(_, work)| work).collect())
}

pub fn set_frame(hwnd: isize, target: Rect) -> Result<()> {
    let win = focused_window(pid(hwnd)).context("포커스된 창이 없다 (손쉬운 사용 권한을 확인하세요)")?;
    let pos = CGPoint { x: target.x as f64, y: target.y as f64 };
    let size = CGSize { width: target.w as f64, height: target.h as f64 };
    // 크기를 먼저 바꾸면 화면 밖으로 밀려 잘리고, 위치를 먼저 바꾸면 큰 창이 다른 화면에 걸쳐 거부된다.
    // 위치 → 크기 → 위치 순으로 두 경우를 모두 피한다
    set_ax_value(win.0, "AXPosition", kAXValueCGPointType, &pos)?;
    set_ax_value(win.0, "AXSize", kAXValueCGSizeType, &size)?;
    set_ax_value(win.0, "AXPosition", kAXValueCGPointType, &pos)
}
