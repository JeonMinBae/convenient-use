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
    let work = window_ops::monitor_at_cursor()?;
    let size = win.outer_size()?;
    let x = work.x + (work.w - size.width as i32) / 2;
    let y = work.y + (work.h - size.height as i32) / 2;
    win.set_position(tauri::PhysicalPosition::new(x, y))?;
    win.show()?;
    win.set_focus()?;
    app.emit_to(POPUP_LABEL, "clipboard-popup-opened", ())?;
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
