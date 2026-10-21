// CoreFoundation·CoreGraphics·Accessibility(HIServices) C API 중 이 앱이 쓰는 것만 직접 선언한다.
#![allow(non_upper_case_globals, dead_code)]

use std::ffi::c_void;

use objc2::encode::{Encode, Encoding};

pub type CFTypeRef = *const c_void;
pub type CFIndex = isize;
pub type CGEventRef = *mut c_void;
pub type CGEventSourceRef = *mut c_void;
pub type CFMachPortRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;
pub type AXUIElementRef = *const c_void;

#[repr(C)]
pub struct Opaque([u8; 0]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

// msg_send! 가 디버그 빌드에서 반환 타입 인코딩을 검사하므로 런타임 이름과 같아야 한다
unsafe impl Encode for CGPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for CGSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for CGRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
}

pub const kCFNumberSInt64Type: CFIndex = 4;
pub const kCFNumberDoubleType: CFIndex = 13;
pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;

pub const kCGWindowListOptionAll: u32 = 0;
pub const kCGWindowListExcludeDesktopElements: u32 = 1 << 4;
pub const kCGNullWindowID: u32 = 0;

pub const kCGSessionEventTap: u32 = 1;
pub const kCGHIDEventTap: u32 = 0;
pub const kCGHeadInsertEventTap: u32 = 0;
pub const kCGEventTapOptionDefault: u32 = 0;

pub const kCGEventKeyDown: u32 = 10;
pub const kCGEventKeyUp: u32 = 11;
pub const kCGEventFlagsChanged: u32 = 12;
pub const kCGEventTapDisabledByTimeout: u32 = 0xFFFF_FFFE;
pub const kCGEventTapDisabledByUserInput: u32 = 0xFFFF_FFFF;

pub const kCGKeyboardEventKeycode: u32 = 9;
pub const kCGEventSourceUserData: u32 = 42;

pub const kCGEventSourceStatePrivate: i32 = -1;
pub const kCGEventSourceStateHIDSystemState: i32 = 1;

pub const kCGEventFlagMaskAlphaShift: u64 = 0x0001_0000;
pub const kCGEventFlagMaskShift: u64 = 0x0002_0000;
pub const kCGEventFlagMaskControl: u64 = 0x0004_0000;
pub const kCGEventFlagMaskAlternate: u64 = 0x0008_0000;
pub const kCGEventFlagMaskCommand: u64 = 0x0010_0000;
pub const kCGEventFlagMaskNumericPad: u64 = 0x0020_0000;
pub const kCGEventFlagMaskSecondaryFn: u64 = 0x0080_0000;

pub const kAXErrorSuccess: i32 = 0;
pub const kAXValueCGPointType: u32 = 1;
pub const kAXValueCGSizeType: u32 = 2;

pub type CGEventTapCallBack =
    unsafe extern "C" fn(proxy: *mut c_void, ty: u32, event: CGEventRef, user: *mut c_void) -> CGEventRef;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub static kCFBooleanTrue: CFTypeRef;
    pub static kCFBooleanFalse: CFTypeRef;
    pub static kCFRunLoopCommonModes: CFTypeRef;
    pub static kCFTypeDictionaryKeyCallBacks: Opaque;
    pub static kCFTypeDictionaryValueCallBacks: Opaque;

    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFArrayGetCount(array: CFTypeRef) -> CFIndex;
    pub fn CFArrayGetValueAtIndex(array: CFTypeRef, idx: CFIndex) -> CFTypeRef;
    pub fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    pub fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        count: CFIndex,
        key_cb: *const Opaque,
        value_cb: *const Opaque,
    ) -> CFTypeRef;
    pub fn CFNumberGetValue(number: CFTypeRef, ty: CFIndex, out: *mut c_void) -> u8;
    pub fn CFBooleanGetValue(b: CFTypeRef) -> u8;
    pub fn CFStringCreateWithBytes(
        alloc: CFTypeRef,
        bytes: *const u8,
        len: CFIndex,
        encoding: u32,
        external: u8,
    ) -> CFTypeRef;
    pub fn CFMachPortCreateRunLoopSource(alloc: CFTypeRef, port: CFMachPortRef, order: CFIndex) -> CFRunLoopSourceRef;
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFTypeRef);
    pub fn CFRunLoopRun();
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub static kCGWindowOwnerPID: CFTypeRef;
    pub static kCGWindowLayer: CFTypeRef;
    pub static kCGWindowBounds: CFTypeRef;
    pub static kCGWindowAlpha: CFTypeRef;

    pub fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
    pub fn CGRectMakeWithDictionaryRepresentation(dict: CFTypeRef, rect: *mut CGRect) -> bool;

    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    pub fn CGEventGetFlags(event: CGEventRef) -> u64;
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventSetType(event: CGEventRef, ty: u32);
    pub fn CGEventGetTimestamp(event: CGEventRef) -> u64;
    pub fn CGEventKeyboardGetUnicodeString(event: CGEventRef, max: usize, actual: *mut usize, buf: *mut u16);
    pub fn CGEventCreateKeyboardEvent(source: CGEventSourceRef, keycode: u16, down: bool) -> CGEventRef;
    pub fn CGEventPost(tap: u32, event: CGEventRef);
    pub fn CGEventSourceCreate(state: i32) -> CGEventSourceRef;
    pub fn CGEventSourceFlagsState(state: i32) -> u64;
    pub fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub static kAXTrustedCheckOptionPrompt: CFTypeRef;

    pub fn AXIsProcessTrusted() -> u8;
    pub fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> u8;
    pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    pub fn AXUIElementCopyAttributeValue(el: AXUIElementRef, attr: CFTypeRef, value: *mut CFTypeRef) -> i32;
    pub fn AXUIElementSetAttributeValue(el: AXUIElementRef, attr: CFTypeRef, value: CFTypeRef) -> i32;
    pub fn AXValueCreate(ty: u32, value: *const c_void) -> CFTypeRef;
    pub fn AXValueGetValue(value: CFTypeRef, ty: u32, out: *mut c_void) -> u8;
}

/// Create/Copy 규칙으로 받은 CF 객체. drop 시 CFRelease 한다.
pub struct Cf(pub CFTypeRef);

impl Cf {
    pub fn new(r: CFTypeRef) -> Option<Self> {
        (!r.is_null()).then_some(Self(r))
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}

pub fn cfstr(s: &str) -> Cf {
    let r = unsafe {
        CFStringCreateWithBytes(std::ptr::null(), s.as_ptr(), s.len() as CFIndex, kCFStringEncodingUTF8, 0)
    };
    Cf(r)
}

pub fn cf_i64(n: CFTypeRef) -> Option<i64> {
    let mut v = 0i64;
    (!n.is_null() && unsafe { CFNumberGetValue(n, kCFNumberSInt64Type, &mut v as *mut i64 as *mut c_void) } != 0)
        .then_some(v)
}

pub fn cf_f64(n: CFTypeRef) -> Option<f64> {
    let mut v = 0f64;
    (!n.is_null() && unsafe { CFNumberGetValue(n, kCFNumberDoubleType, &mut v as *mut f64 as *mut c_void) } != 0)
        .then_some(v)
}

/// 손쉬운 사용 권한 여부. prompt 면 미허용 시 시스템 설정 안내 대화상자를 띄운다.
pub fn ax_trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let Some(opts) = Cf::new(CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        )) else {
            return AXIsProcessTrusted() != 0;
        };
        AXIsProcessTrustedWithOptions(opts.0) != 0
    }
}
