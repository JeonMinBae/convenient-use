use std::io::Cursor;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext, ContentFormat,
    common::RustImage,
};
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use super::store::ClipContent;
use crate::state::AppState;

const THUMB_MAX_SIDE: u32 = 160;

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// 전용 스레드에서 클립보드 변경을 감시한다 (start_watch 는 블로킹).
pub fn start(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("clipboard-watcher".into()).spawn(move || {
        let mut watcher = match ClipboardWatcherContext::new() {
            Ok(w) => w,
            Err(e) => return log::warn!("클립보드 감시 시작 실패: {e}"),
        };
        watcher.add_handler(Handler { app });
        watcher.start_watch();
    });
    if let Err(e) = spawned {
        log::warn!("클립보드 감시 스레드 생성 실패: {e}");
    }
}

struct Handler {
    app: AppHandle,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) {
        let state = self.app.state::<AppState>();
        let (enabled, max_items, max_image_mb) = {
            let s = state.settings.read().unwrap();
            (s.general.features.clipboard, s.clipboard.max_items, s.clipboard.max_image_mb)
        };
        if !enabled {
            return;
        }
        // 앱이 직접 클립보드를 바꾸는 구간(스니펫 임시 복사 등)의 변경은 기록하지 않는다
        if state.clip_suppress_until.lock().unwrap().is_some_and(|t| Instant::now() < t) {
            return;
        }
        let Some(content) = read_current(max_image_mb) else { return };
        if let Err(e) = state.clips.lock().unwrap().record(content, now_ms(), max_items) {
            log::warn!("클립보드 기록 실패: {e}");
        }
    }
}

/// files > image > text 순. 다른 프로그램이 클립보드를 잡고 있을 수 있어 짧게 재시도한다.
fn read_current(max_image_mb: u32) -> Option<ClipContent> {
    let ctx = ClipboardContext::new().ok()?;
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(50));
        }
        if ctx.has(ContentFormat::Files)
            && let Ok(files) = ctx.get_files()
                && !files.is_empty() {
                    return Some(ClipContent::Files(files));
                }
        if ctx.has(ContentFormat::Image)
            && let Ok(img) = ctx.get_image()
                && let Ok(rgba) = img.to_rgba8() {
                    return build_image(rgba, max_image_mb);
                }
        if ctx.has(ContentFormat::Text)
            && let Ok(text) = ctx.get_text()
                && !text.is_empty() {
                    return Some(ClipContent::Text(text));
                }
    }
    None
}

/// 너비·높이를 함께 해시한다. 바이트가 같아도 크기가 다르면 다른 이미지다.
fn rgba_hash(rgba: &RgbaImage) -> String {
    let mut h = Sha256::new();
    h.update(rgba.width().to_le_bytes());
    h.update(rgba.height().to_le_bytes());
    h.update(rgba.as_raw());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// PNG 가 max_image_mb 를 넘으면 None.
fn build_image(rgba: RgbaImage, max_image_mb: u32) -> Option<ClipContent> {
    let rgba_hash = rgba_hash(&rgba);
    let img = DynamicImage::ImageRgba8(rgba);
    let png = encode_png(&img)?;
    if png.len() as u64 > max_image_mb as u64 * 1024 * 1024 {
        return None;
    }
    let thumb = if img.width().max(img.height()) > THUMB_MAX_SIDE {
        img.resize(THUMB_MAX_SIDE, THUMB_MAX_SIDE, FilterType::Triangle)
    } else {
        img
    };
    Some(ClipContent::Image { png, thumb_png: encode_png(&thumb)?, rgba_hash })
}

fn encode_png(img: &DynamicImage) -> Option<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_gets_160px_thumbnail_and_size_limit() {
        let rgba = RgbaImage::from_pixel(400, 200, image::Rgba([10, 20, 30, 255]));
        let Some(ClipContent::Image { png, thumb_png, .. }) = build_image(rgba.clone(), 10) else { panic!() };
        assert!(!png.is_empty());
        let thumb = image::load_from_memory(&thumb_png).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (160, 80));
        assert!(build_image(rgba, 0).is_none());
    }

    #[test]
    fn hash_distinguishes_dimensions_with_same_bytes() {
        let px = image::Rgba([1, 2, 3, 255]);
        let a = RgbaImage::from_pixel(100, 200, px);
        let b = RgbaImage::from_pixel(200, 100, px);
        assert_eq!(a.as_raw(), b.as_raw());
        assert_ne!(rgba_hash(&a), rgba_hash(&b));
        assert_eq!(rgba_hash(&a), rgba_hash(&a.clone()));
    }
}
