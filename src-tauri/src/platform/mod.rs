#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
#[allow(unused_imports)]
pub use self::windows as current;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub use self::macos as current;

#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;
#[cfg(not(any(windows, target_os = "macos")))]
#[allow(unused_imports)]
pub use self::unsupported as current;
