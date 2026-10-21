use anyhow::{Context, anyhow};
use clipboard_rs::{Clipboard, ClipboardContext, common::RustImage, common::RustImageData};
use tauri::{AppHandle, Emitter, Manager};

use super::store::ClipItem;
use crate::platform::current::{input, process, window_ops};
use crate::state::AppState;

const POPUP_LABEL: &str = "clipboard-popup";

pub fn show(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    *state.popup_prev.lock().unwrap() = window_ops::foreground().ok();
    let win = app.get_webview_window(POPUP_LABEL).context("popup window")?;
    center_in(&win, window_ops::monitor_at_cursor()?)?;
    win.show()?;
    win.set_focus()?;
    app.emit_to(POPUP_LABEL, "clipboard-popup-opened", ())?;
    Ok(())
}

/// 작업 영역 좌표는 Windows 가 물리 픽셀, macOS 가 포인트(논리 픽셀)다.
fn center_in(win: &tauri::WebviewWindow, work: crate::window_layout::Rect) -> anyhow::Result<()> {
    let size = win.outer_size()?;
    #[cfg(target_os = "macos")]
    let size = size.to_logical::<u32>(win.scale_factor()?);
    let x = work.x + (work.w - size.width as i32) / 2;
    let y = work.y + (work.h - size.height as i32) / 2;
    #[cfg(target_os = "macos")]
    win.set_position(tauri::LogicalPosition::new(x, y))?;
    #[cfg(not(target_os = "macos"))]
    win.set_position(tauri::PhysicalPosition::new(x, y))?;
    Ok(())
}

pub fn hide(app: &AppHandle) -> anyhow::Result<()> {
    app.get_webview_window(POPUP_LABEL).context("popup window")?.hide()?;
    Ok(())
}

pub fn paste(app: &AppHandle, id: i64) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let (clip, image_path) = state.clips.lock().unwrap().get(id)?.ok_or_else(|| anyhow!("클립 {id} 없음"))?;
    let _op = super::op_lock();
    write_to_clipboard(&clip, image_path.as_deref())?;
    hide(app)?;
    let prev = *state.popup_prev.lock().unwrap();
    if let Some(prev) = prev {
        process::focus(prev)?;
    }
    // 포커스 전환이 끝나기 전에 Ctrl+V 가 가면 팝업/이전 창이 아닌 곳에 입력된다
    std::thread::sleep(std::time::Duration::from_millis(50));
    input::send_paste()
}

fn write_to_clipboard(clip: &ClipItem, image_path: Option<&std::path::Path>) -> anyhow::Result<()> {
    let ctx = ClipboardContext::new().map_err(|e| anyhow!("클립보드 열기 실패: {e}"))?;
    let result = match clip.kind.as_str() {
        "image" => {
            let path = image_path.context("이미지 파일 경로 없음")?;
            let data = RustImageData::from_bytes(&std::fs::read(path)?).map_err(|e| anyhow!("{e}"))?;
            ctx.set_image(data)
        }
        "files" => ctx.set_files(clip.files.clone().unwrap_or_default()),
        _ => ctx.set_text(clip.text.clone().unwrap_or_default()),
    };
    result.map_err(|e| anyhow!("클립보드 쓰기 실패: {e}"))
}
