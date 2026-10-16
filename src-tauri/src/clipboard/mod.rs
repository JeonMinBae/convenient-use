pub mod popup;
pub mod store;
pub mod watcher;

use std::sync::{Mutex, MutexGuard};

/// 클립보드를 임시로 바꾸는 작업(스니펫 확장, 팝업 붙여넣기)은 이 락을 전 구간 잡고 직렬화한다.
/// 겹치면 한쪽이 상대의 임시 내용을 원본으로 저장했다가 복원해 버린다.
static OP_LOCK: Mutex<()> = Mutex::new(());

pub fn op_lock() -> MutexGuard<'static, ()> {
    OP_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
