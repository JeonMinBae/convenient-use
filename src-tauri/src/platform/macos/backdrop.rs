use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

/// 설정 창은 사이드바, 팝업은 HUD 재질. 투명 창이 필요하므로 tauri.conf 의 macOSPrivateApi 가 켜져 있어야 한다.
/// 메인 스레드에서 호출해야 한다 (동기 커맨드는 메인 스레드에서 실행된다).
pub fn apply(window: &tauri::WebviewWindow) -> bool {
    let material = if window.label() == "clipboard-popup" {
        NSVisualEffectMaterial::HudWindow
    } else {
        NSVisualEffectMaterial::Sidebar
    };
    // 프레임 없는 창은 모서리가 각지므로 재질 뷰에 둥근 모서리를 준다
    apply_vibrancy(window, material, Some(NSVisualEffectState::Active), Some(10.0)).is_ok()
}
