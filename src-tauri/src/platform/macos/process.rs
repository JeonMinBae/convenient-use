// 핸들(isize)은 pid 다. 실행 파일 경로 대신 앱 번들 경로("/Applications/Slack.app")를 쓴다.
use anyhow::{Context, Result, bail};

use super::ffi::*;
use super::{appkit, window_ops};

/// 창이 있는 일반 앱을 Z-order(앞→뒤) 순으로 (pid, 번들 경로) 로 돌려준다. 앱마다 한 번만 나온다.
/// 최소화·숨김·다른 Space 의 창도 포함한다 (그런 앱은 새로 실행하지 않고 앞으로 가져온다).
pub fn top_windows() -> Result<Vec<(isize, String)>> {
    let list = Cf::new(unsafe {
        CGWindowListCopyWindowInfo(kCGWindowListOptionAll | kCGWindowListExcludeDesktopElements, kCGNullWindowID)
    })
    .context("CGWindowListCopyWindowInfo 실패")?;
    let mut pids: Vec<i32> = Vec::new();
    let n = unsafe { CFArrayGetCount(list.0) };
    for i in 0..n {
        let dict = unsafe { CFArrayGetValueAtIndex(list.0, i) };
        let get = |key: CFTypeRef| unsafe { CFDictionaryGetValue(dict, key) };
        // 레이어 0 이 일반 문서 창. 메뉴 막대·팝오버 등은 다른 레이어다
        if cf_i64(get(unsafe { kCGWindowLayer })) != Some(0) || cf_f64(get(unsafe { kCGWindowAlpha })) == Some(0.0) {
            continue;
        }
        let mut bounds = CGRect::default();
        let b = get(unsafe { kCGWindowBounds });
        // 툴팁·보이지 않는 보조 창을 거른다
        if b.is_null()
            || !unsafe { CGRectMakeWithDictionaryRepresentation(b, &mut bounds) }
            || bounds.size.width < 50.0
            || bounds.size.height < 50.0
        {
            continue;
        }
        if let Some(pid) = cf_i64(get(unsafe { kCGWindowOwnerPID })).map(|p| p as i32)
            && !pids.contains(&pid)
        {
            pids.push(pid);
        }
    }
    Ok(pids
        .into_iter()
        .filter(|&p| appkit::is_regular_app(p))
        .filter_map(|p| appkit::bundle_path(p).map(|path| (p as isize, path)))
        .collect())
}

/// 포그라운드 앱의 번들 파일 이름 (소문자, 예: "safari.app")
pub fn foreground_exe() -> Result<String> {
    let pid = appkit::frontmost_pid().context("포그라운드 앱이 없다")?;
    let path = appkit::bundle_path(pid).context("앱 번들을 알 수 없다")?;
    Ok(path.trim_end_matches('/').rsplit('/').next().unwrap_or(&path).to_lowercase())
}

/// 앱을 앞으로 가져오고, 포커스 창이 최소화돼 있으면 복원한다.
pub fn focus(hwnd: isize) -> Result<()> {
    let pid = hwnd as i32;
    if !appkit::activate(pid) {
        bail!("앱 활성화 실패 (pid {pid})");
    }
    if let Some(win) = window_ops::focused_window(pid)
        && window_ops::ax_attr(win.0, "AXMinimized").is_some_and(|v| unsafe { CFBooleanGetValue(v.0) } != 0)
    {
        unsafe { AXUIElementSetAttributeValue(win.0, cfstr("AXMinimized").0, kCFBooleanFalse) };
    }
    Ok(())
}

/// macOS 관례대로 창 하나를 최소화하는 대신 앱 전체를 가린다 (Cmd+H 와 같다).
pub fn minimize(hwnd: isize) -> Result<()> {
    appkit::hide(hwnd as i32);
    Ok(())
}
