use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};

/// 설정 창은 Mica, 팝업은 Acrylic. 적용에 실패하면(Win10 등) false 를 돌려 프론트가 단색 배경을 쓰게 한다.
/// Mica 의 라이트/다크는 tao 가 시스템 테마로 설정한 DWMWA_USE_IMMERSIVE_DARK_MODE 를 따르므로 dark 인자는 넘기지 않는다.
pub fn apply(window: &tauri::WebviewWindow) -> bool {
    let applied = if window.label() == "clipboard-popup" {
        window_vibrancy::apply_acrylic(window, None).is_ok()
    } else {
        window_vibrancy::apply_mica(window, None).is_ok()
    };
    if window.label() == "clipboard-popup" {
        // 프레임 없는 창도 Win11 둥근 모서리를 갖게 한다. 실패해도(Win10) 동작에는 지장 없다
        if let Ok(h) = window.hwnd() {
            let pref = DWMWCP_ROUND;
            unsafe {
                let _ = DwmSetWindowAttribute(
                    HWND(h.0 as _), DWMWA_WINDOW_CORNER_PREFERENCE,
                    &pref as *const _ as _, std::mem::size_of_val(&pref) as u32,
                );
            }
        }
    }
    applied
}
