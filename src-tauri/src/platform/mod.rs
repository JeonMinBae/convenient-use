#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
#[allow(unused_imports)]
pub use self::windows as current;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
#[allow(unused_imports)]
pub use self::unsupported as current;
