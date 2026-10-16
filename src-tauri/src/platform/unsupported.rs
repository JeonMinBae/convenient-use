// macOS 등 미지원 OS 의 컴파일 경계를 유지하는 스텁. windows 모듈과 같은 함수 시그니처를 가진다.

pub mod window_ops {
    use crate::window_layout::Rect;

    pub fn foreground() -> anyhow::Result<isize> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn frame(_hwnd: isize) -> anyhow::Result<Rect> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn monitor_work(_hwnd: isize) -> anyhow::Result<Rect> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn monitor_at_cursor() -> anyhow::Result<Rect> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn monitors() -> anyhow::Result<Vec<Rect>> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn set_frame(_hwnd: isize, _target: Rect) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
}

pub mod process {
    pub fn top_windows() -> anyhow::Result<Vec<(isize, String)>> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn foreground_exe() -> anyhow::Result<String> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn focus(_hwnd: isize) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn minimize(_hwnd: isize) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
}

pub mod input {
    use crate::remap::KeyCombo;

    pub const INJECTED_MARK: usize = 0x434F_4E56;

    pub fn send_keys(_combos: &[KeyCombo]) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn send_events(_events: &[(String, bool)]) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn held_modifiers() -> Vec<String> {
        Vec::new()
    }
    pub fn send_paste() -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
}

pub mod hook {
    use crate::keyboard::KeyEvent;

    pub fn install(_cb: fn(&KeyEvent) -> bool) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
}
