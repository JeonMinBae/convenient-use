// macOS 창·프로세스·입력·훅 호출은 이 모듈 아래에만 둔다. windows 모듈과 같은 공개 함수 시그니처를 가진다.
// 창 핸들(isize)은 앱 pid 이고, 대부분의 기능에 손쉬운 사용 권한이 필요하다.
mod appkit;
mod ffi;
mod keycodes;
pub mod window_ops;
pub mod process;
pub mod input;
pub mod hook;
pub mod backdrop;
