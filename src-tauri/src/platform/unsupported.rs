// macOS 등 미지원 OS 의 컴파일 경계. 이후 단위에서 windows 모듈과 같은 함수 시그니처의 스텁을 채운다.

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
    pub fn focus(_hwnd: isize) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
    pub fn minimize(_hwnd: isize) -> anyhow::Result<()> {
        anyhow::bail!("지원하지 않는 OS")
    }
}
