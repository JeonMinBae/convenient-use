//! 전역 키보드 훅과 핸들러 체인. 리매핑·스니펫 등은 `KeyHandler` 로 등록한다.
pub mod keys;

use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq)]
pub struct KeyEvent {
    /// W3C code ("KeyA"). IME 처리 중이면 "Process", 표에 없는 키는 "Unknown"
    pub code: String,
    pub down: bool,
    pub time_ms: u64,
    /// 이 앱이 SendInput 으로 보낸 이벤트(INJECTED_MARK)만 true
    pub injected: bool,
    /// 현재 레이아웃에서 이 키가 만드는 문자. key-down 에서만 계산한다
    pub ch: Option<char>,
}

impl KeyEvent {
    #[cfg(test)]
    pub fn key(code: &str, down: bool) -> Self {
        Self { code: code.to_string(), down, time_ms: 0, injected: false, ch: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDecision {
    Pass,
    Suppress,
}

/// 훅 콜백 안에서 호출되므로 빨리 반환해야 한다(OS 타임아웃 시 훅이 해제된다). 대기가 필요한 동작은 별도 스레드로 보낸다.
pub trait KeyHandler: Send {
    fn on_key(&mut self, ev: &KeyEvent) -> KeyDecision;
}

pub struct Dispatcher {
    handlers: Vec<Box<dyn KeyHandler>>,
}

impl Dispatcher {
    pub fn new(handlers: Vec<Box<dyn KeyHandler>>) -> Self {
        Self { handlers }
    }

    pub fn handle(&mut self, ev: &KeyEvent) -> KeyDecision {
        // 자기 주입 이벤트는 핸들러에 다시 넣지 않는다 (리매핑 출력이 재리매핑되는 루프 방지)
        if ev.injected {
            return KeyDecision::Pass;
        }
        for h in &mut self.handlers {
            if h.on_key(ev) == KeyDecision::Suppress {
                return KeyDecision::Suppress;
            }
        }
        KeyDecision::Pass
    }
}

fn dispatcher() -> &'static Mutex<Dispatcher> {
    static D: OnceLock<Mutex<Dispatcher>> = OnceLock::new();
    D.get_or_init(|| Mutex::new(Dispatcher::new(Vec::new())))
}

/// 핸들러 목록을 통째로 교체한다 (설정 변경 시 호출).
pub fn set_handlers(handlers: Vec<Box<dyn KeyHandler>>) {
    let mut d = dispatcher().lock().unwrap_or_else(|e| e.into_inner());
    d.handlers = handlers;
}

fn on_event(ev: &KeyEvent) -> bool {
    // 핸들러가 훅 스레드에서 SendInput 하면 훅이 같은 스레드로 재진입할 수 있다.
    // 자기 주입은 락 없이 통과시키고, 그 외 재진입·설정 교체 중에는 블로킹 대신 통과시킨다.
    if ev.injected {
        return false;
    }
    match dispatcher().try_lock() {
        Ok(mut d) => d.handle(ev) == KeyDecision::Suppress,
        Err(_) => false,
    }
}

pub fn start() {
    if let Err(e) = crate::platform::current::hook::install(on_event) {
        log::error!("키보드 훅 설치 실패: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct Rec(Arc<Mutex<Vec<&'static str>>>, &'static str, KeyDecision);
    impl KeyHandler for Rec {
        fn on_key(&mut self, _: &KeyEvent) -> KeyDecision {
            self.0.lock().unwrap().push(self.1);
            self.2
        }
    }

    fn key(code: &str, down: bool) -> KeyEvent {
        KeyEvent::key(code, down)
    }

    #[test]
    fn suppress_stops_chain_and_injected_skips_all() {
        let log = Arc::new(Mutex::new(vec![]));
        let mut d = Dispatcher::new(vec![
            Box::new(Rec(log.clone(), "remap", KeyDecision::Suppress)),
            Box::new(Rec(log.clone(), "snippet", KeyDecision::Pass)),
        ]);
        assert_eq!(d.handle(&key("KeyA", true)), KeyDecision::Suppress);
        assert_eq!(d.handle(&KeyEvent { injected: true, ..key("KeyA", true) }), KeyDecision::Pass);
        assert_eq!(*log.lock().unwrap(), vec!["remap"]);
    }

    #[test]
    fn pass_runs_handlers_in_order() {
        let log = Arc::new(Mutex::new(vec![]));
        let mut d = Dispatcher::new(vec![
            Box::new(Rec(log.clone(), "a", KeyDecision::Pass)),
            Box::new(Rec(log.clone(), "b", KeyDecision::Pass)),
        ]);
        assert_eq!(d.handle(&key("KeyA", true)), KeyDecision::Pass);
        assert_eq!(*log.lock().unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn empty_dispatcher_passes() {
        assert_eq!(Dispatcher::new(vec![]).handle(&key("KeyA", true)), KeyDecision::Pass);
    }
}
