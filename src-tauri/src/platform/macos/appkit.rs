// NSWorkspace·NSRunningApplication·NSScreen 호출. 훅·단축키 스레드에서도 부르므로 매번 autoreleasepool 로 감싼다.
use std::ffi::{CStr, c_char};

use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

use super::ffi::{CGPoint, CGRect};

const NS_APPLICATION_ACTIVATE_ALL_WINDOWS: usize = 1 << 0;
const NS_APPLICATION_ACTIVATE_IGNORING_OTHER_APPS: usize = 1 << 1;
const NS_APPLICATION_ACTIVATION_POLICY_REGULAR: isize = 0;

fn running_app(pid: i32) -> Option<Retained<AnyObject>> {
    unsafe { msg_send![class!(NSRunningApplication), runningApplicationWithProcessIdentifier: pid] }
}

fn ns_string(s: &AnyObject) -> String {
    let p: *const c_char = unsafe { msg_send![s, UTF8String] };
    if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
}

pub fn frontmost_pid() -> Option<i32> {
    autoreleasepool(|_| unsafe {
        let ws: Retained<AnyObject> = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: Option<Retained<AnyObject>> = msg_send![&*ws, frontmostApplication];
        app.map(|a| msg_send![&*a, processIdentifier])
    })
}

/// 앱 번들 경로 (예: "/Applications/Slack.app"). 번들이 없는 프로세스는 None.
pub fn bundle_path(pid: i32) -> Option<String> {
    autoreleasepool(|_| unsafe {
        let app = running_app(pid)?;
        let url: Option<Retained<AnyObject>> = msg_send![&*app, bundleURL];
        let url = url?;
        let path: Option<Retained<AnyObject>> = msg_send![&*url, path];
        path.map(|p| ns_string(&p))
    })
}

/// Dock 에 나오는 일반 앱인지 (메뉴 막대 상주·백그라운드 에이전트 제외)
pub fn is_regular_app(pid: i32) -> bool {
    autoreleasepool(|_| unsafe {
        running_app(pid).is_some_and(|a| {
            let policy: isize = msg_send![&*a, activationPolicy];
            policy == NS_APPLICATION_ACTIVATION_POLICY_REGULAR
        })
    })
}

/// 숨김 상태면 해제한 뒤 모든 창을 앞으로 가져온다.
pub fn activate(pid: i32) -> bool {
    autoreleasepool(|_| unsafe {
        let Some(app) = running_app(pid) else { return false };
        let hidden: bool = msg_send![&*app, isHidden];
        if hidden {
            let _: bool = msg_send![&*app, unhide];
        }
        // IgnoringOtherApps 는 macOS 14 부터 무시되지만 이전 버전에서는 있어야 상주 앱에서 전환된다
        let opts = NS_APPLICATION_ACTIVATE_ALL_WINDOWS | NS_APPLICATION_ACTIVATE_IGNORING_OTHER_APPS;
        let ok: bool = msg_send![&*app, activateWithOptions: opts];
        ok
    })
}

pub fn hide(pid: i32) -> bool {
    autoreleasepool(|_| unsafe { running_app(pid).is_some_and(|a| msg_send![&*a, hide]) })
}

/// 각 화면의 (전체 프레임, 메뉴 막대·Dock 을 뺀 영역). Cocoa 좌표계(주 화면 왼쪽 아래 원점, y 위쪽)다.
/// 첫 번째가 주 화면이다.
pub fn screens() -> Vec<(CGRect, CGRect)> {
    autoreleasepool(|_| unsafe {
        let screens: Option<Retained<AnyObject>> = msg_send![class!(NSScreen), screens];
        let Some(screens) = screens else { return Vec::new() };
        let n: usize = msg_send![&*screens, count];
        (0..n)
            .map(|i| {
                let s: Retained<AnyObject> = msg_send![&*screens, objectAtIndex: i];
                let frame: CGRect = msg_send![&*s, frame];
                let visible: CGRect = msg_send![&*s, visibleFrame];
                (frame, visible)
            })
            .collect()
    })
}

/// Cocoa 좌표계의 마우스 위치.
pub fn mouse_location() -> CGPoint {
    unsafe { msg_send![class!(NSEvent), mouseLocation] }
}
