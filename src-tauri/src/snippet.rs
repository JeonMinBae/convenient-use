//! 텍스트 스니펫 자동 확장. 키워드 입력이 끝나는 즉시(구분자 없이) 치환한다.
use std::collections::HashSet;
use std::time::{Duration, Instant};

use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat, common::RustImageData};
use tauri::{AppHandle, Manager};

use crate::keyboard::{KeyDecision, KeyEvent, KeyHandler};
use crate::platform::current::{input, window_ops};
use crate::remap::KeyCombo;
use crate::settings::model::Snippet;
use crate::settings::validate::ValidationError;
use crate::state::AppState;

const MIN_LEN: usize = 2;
const MAX_LEN: usize = 32;

#[derive(Debug, PartialEq)]
pub enum Feed {
    Pass,
    Expand { erase: usize, text: String },
}

/// 순수 상태 기계: OS 호출 없음.
pub struct Matcher {
    keywords: Vec<(String, String)>,
    buf: String,
}

fn is_modifier(code: &str) -> bool {
    matches!(
        code,
        "ShiftLeft" | "ShiftRight" | "ControlLeft" | "ControlRight" | "AltLeft" | "AltRight" | "MetaLeft" | "MetaRight"
    )
}

impl Matcher {
    // 키워드 최대 길이와 같다: 더 길게 보관할 필요가 없다
    const MAX_BUF: usize = MAX_LEN;

    pub fn new(keywords: Vec<(String, String)>) -> Self {
        Self { keywords, buf: String::new() }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn feed(&mut self, ev: &KeyEvent, has_cmd_mod: bool) -> Feed {
        if !ev.down || is_modifier(&ev.code) {
            return Feed::Pass;
        }
        if ev.code == "Backspace" {
            self.buf.pop();
            return Feed::Pass;
        }
        // ch 가 없는 키(방향키·Enter·IME "Process" 등)나 Ctrl/Alt/Super 조합은 입력 위치가 바뀌었다는 뜻
        let Some(ch) = ev.ch.filter(|c| !has_cmd_mod && !c.is_control()) else {
            self.buf.clear();
            return Feed::Pass;
        };
        self.buf.push(ch);
        let excess = self.buf.chars().count().saturating_sub(Self::MAX_BUF);
        if excess > 0 {
            self.buf = self.buf.chars().skip(excess).collect();
        }
        let hit = self
            .keywords
            .iter()
            .filter(|(k, _)| self.buf.ends_with(k.as_str()))
            .max_by_key(|(k, _)| k.len());
        match hit {
            Some((k, text)) => {
                let erase = k.chars().count() - 1; // 마지막 글자는 Suppress 되어 입력되지 않는다
                let text = text.clone();
                self.buf.clear();
                Feed::Expand { erase, text }
            }
            None => Feed::Pass,
        }
    }
}

pub fn validate_keywords(snippets: &[Snippet]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let err = |s: &Snippet, message: String| ValidationError { field: format!("snippets.{}.keyword", s.id), message };
    for (i, s) in snippets.iter().enumerate() {
        let k = &s.keyword;
        let len = k.chars().count();
        if k.is_empty() {
            errors.push(err(s, "키워드를 입력하세요".into()));
        } else if !(MIN_LEN..=MAX_LEN).contains(&len) {
            errors.push(err(s, format!("키워드는 {MIN_LEN}~{MAX_LEN}자여야 합니다")));
        } else if !k.chars().all(|c| c.is_ascii_graphic()) {
            errors.push(err(s, "키워드는 공백 없는 영문·숫자·기호(ASCII)만 쓸 수 있습니다".into()));
        } else if snippets.iter().enumerate().any(|(j, o)| j != i && o.keyword == *k) {
            errors.push(err(s, "중복된 키워드입니다".into()));
        } else if let Some(o) = snippets.iter().find(|o| o.keyword != *k && o.keyword.starts_with(k.as_str())) {
            // 즉시 치환이라 짧은 키워드에서 먼저 확장되어 긴 쪽에 도달할 수 없다
            errors.push(err(s, format!("다른 키워드 '{}' 의 앞부분입니다", o.keyword)));
        }
    }
    errors
}

pub struct SnippetHandler {
    matcher: Matcher,
    app: AppHandle,
    last_fg: Option<isize>,
    /// 눌려 있는 Ctrl/Alt/Super 키 (이벤트 스트림으로 추적)
    cmd_down: HashSet<String>,
}

impl SnippetHandler {
    pub fn new(app: AppHandle, snippets: &[Snippet]) -> Self {
        // 검증을 통과하지 못한 항목(수동 편집된 설정 파일 등)은 무시한다
        let keywords = snippets
            .iter()
            .filter(|s| validate_keywords(std::slice::from_ref(s)).is_empty())
            .map(|s| (s.keyword.clone(), s.text.clone()))
            .collect();
        Self { matcher: Matcher::new(keywords), app, last_fg: None, cmd_down: HashSet::new() }
    }

    fn track_modifiers(&mut self, ev: &KeyEvent) {
        if matches!(ev.code.as_str(), "ControlLeft" | "ControlRight" | "AltLeft" | "AltRight" | "MetaLeft" | "MetaRight") {
            if ev.down {
                self.cmd_down.insert(ev.code.clone());
            } else {
                self.cmd_down.remove(&ev.code);
            }
        }
    }
}

impl KeyHandler for SnippetHandler {
    fn on_key(&mut self, ev: &KeyEvent) -> KeyDecision {
        self.track_modifiers(ev);
        if ev.down {
            let fg = window_ops::foreground().ok();
            if fg != self.last_fg {
                self.last_fg = fg;
                self.matcher.clear();
            }
        }
        match self.matcher.feed(ev, !self.cmd_down.is_empty()) {
            Feed::Pass => KeyDecision::Pass,
            Feed::Expand { erase, text } => {
                // 훅 콜백에서 SendInput·클립보드 작업을 하면 훅이 막히므로 별도 스레드에서 처리한다
                let app = self.app.clone();
                std::thread::spawn(move || {
                    if let Err(e) = expand(&app, erase, &text) {
                        log::warn!("스니펫 확장 실패: {e}");
                    }
                });
                KeyDecision::Suppress
            }
        }
    }
}

enum Saved {
    Files(Vec<String>),
    Image(RustImageData),
    Text(String),
    Empty,
}

fn save(ctx: &ClipboardContext) -> Saved {
    if ctx.has(ContentFormat::Files)
        && let Ok(f) = ctx.get_files() {
            return Saved::Files(f);
        }
    if ctx.has(ContentFormat::Image)
        && let Ok(i) = ctx.get_image() {
            return Saved::Image(i);
        }
    if ctx.has(ContentFormat::Text)
        && let Ok(t) = ctx.get_text() {
            return Saved::Text(t);
        }
    Saved::Empty
}

fn restore(ctx: &ClipboardContext, saved: Saved) {
    let r = match saved {
        Saved::Files(f) => ctx.set_files(f),
        Saved::Image(i) => ctx.set_image(i),
        Saved::Text(t) => ctx.set_text(t),
        Saved::Empty => ctx.clear(),
    };
    if let Err(e) = r {
        log::warn!("클립보드 복원 실패: {e}");
    }
}

fn expand(app: &AppHandle, erase: usize, text: &str) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    // 임시 텍스트와 복원이 히스토리에 기록되지 않도록 복원 이후까지 억제한다
    *state.clip_suppress_until.lock().unwrap() = Some(Instant::now() + Duration::from_secs(1));
    let ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("클립보드 열기 실패: {e}"))?;
    let saved = save(&ctx);
    let result = (|| {
        ctx.set_text(text.to_string()).map_err(|e| anyhow::anyhow!("클립보드 쓰기 실패: {e}"))?;
        let backspace = KeyCombo { mods: vec![], key: Some("Backspace".into()) };
        input::send_keys(&vec![backspace; erase])?;
        input::send_paste()?;
        // 붙여넣기를 받는 앱이 클립보드를 읽기 전에 복원하면 원래 내용이 붙는다
        std::thread::sleep(Duration::from_millis(150));
        Ok(())
    })();
    restore(&ctx, saved);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn char_event(c: char) -> KeyEvent {
        KeyEvent { ch: Some(c), ..KeyEvent::key("KeyA", true) }
    }
    fn key(code: &str, down: bool) -> KeyEvent {
        KeyEvent::key(code, down)
    }
    fn typed(m: &mut Matcher, s: &str) -> Vec<Feed> {
        s.chars().map(|c| m.feed(&char_event(c), false)).collect()
    }
    fn snippet(k: &str, t: &str) -> Snippet {
        Snippet { id: k.into(), keyword: k.into(), text: t.into() }
    }

    #[test]
    fn expands_immediately_on_last_char() {
        let mut m = Matcher::new(vec![(";addr".into(), "서울시".into())]);
        let out = typed(&mut m, "x;addr");
        assert!(matches!(out.last(), Some(Feed::Expand { erase: 4, text }) if text == "서울시"));
        assert!(out[..5].iter().all(|f| *f == Feed::Pass));
    }

    #[test]
    fn longest_keyword_wins() {
        let mut m = Matcher::new(vec![("dr".into(), "A".into()), (";addr".into(), "B".into())]);
        assert!(matches!(typed(&mut m, ";addr").last(), Some(Feed::Expand { text, .. }) if text == "B"));
    }

    #[test]
    fn backspace_and_navigation_affect_buffer() {
        let mut m = Matcher::new(vec![(";ab".into(), "X".into())]);
        typed(&mut m, ";a");
        m.feed(&key("Backspace", true), false);
        // ";a" 에서 지웠으므로 버퍼는 ";" — "b" 만으로는 매치되지 않고, "ab" 를 다시 치면 매치된다
        assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Pass)));
        m.clear();
        typed(&mut m, ";a");
        m.feed(&key("Backspace", true), false);
        assert!(matches!(typed(&mut m, "ab").last(), Some(Feed::Expand { .. })));
        typed(&mut m, ";a");
        m.feed(&key("ArrowLeft", true), false);
        assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Pass)));
    }

    #[test]
    fn ime_process_key_resets_buffer() {
        let mut m = Matcher::new(vec![(";ab".into(), "X".into())]);
        typed(&mut m, ";a");
        m.feed(&key("Process", true), false);
        assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Pass)));
    }

    #[test]
    fn modifier_alone_and_key_up_keep_buffer_but_cmd_combo_resets() {
        let mut m = Matcher::new(vec![(";ab".into(), "X".into())]);
        typed(&mut m, ";a");
        m.feed(&key("ShiftLeft", true), false);
        m.feed(&key("KeyA", false), false);
        assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Expand { .. })));
        typed(&mut m, ";a");
        m.feed(&char_event('c'), true);
        assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Pass)));
    }

    #[test]
    fn prefix_keywords_rejected() {
        let errs = validate_keywords(&[snippet(";a", "x"), snippet(";ab", "y")]);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].field, "snippets.;a.keyword");
    }

    #[test]
    fn keyword_format_rules() {
        assert!(validate_keywords(&[snippet(";addr", "x")]).is_empty());
        for bad in ["", ";", &";".repeat(33), "가나다", "a b"] {
            assert_eq!(validate_keywords(&[snippet(bad, "x")]).len(), 1, "{bad:?}");
        }
        assert!(validate_keywords(&[snippet(&";".repeat(32), "x")]).is_empty());
    }

    #[test]
    fn duplicates_mark_every_field() {
        let a = Snippet { id: "1".into(), ..snippet(";dup", "x") };
        let b = Snippet { id: "2".into(), ..snippet(";dup", "y") };
        let fields: Vec<_> = validate_keywords(&[a, b]).into_iter().map(|e| e.field).collect();
        assert_eq!(fields, ["snippets.1.keyword", "snippets.2.keyword"]);
    }
}
