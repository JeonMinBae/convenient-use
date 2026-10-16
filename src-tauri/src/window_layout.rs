use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutCommand {
    LeftHalf, RightHalf, TopHalf, BottomHalf,
    TopLeft, TopRight, BottomLeft, BottomRight,
    FirstThird, CenterThird, LastThird, FirstTwoThirds, LastTwoThirds,
    Maximize, Center, NextDisplay,
}

/// 화면 좌표계 사각형 (물리 픽셀).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn compute(cmd: LayoutCommand, a: Rect, current: Rect) -> Rect {
    use LayoutCommand::*;
    let (hw, hh) = (a.w / 2, a.h / 2);
    let third = a.w / 3;
    match cmd {
        LeftHalf => Rect { w: hw, ..a },
        RightHalf => Rect { x: a.x + hw, w: a.w - hw, ..a },
        TopHalf => Rect { h: hh, ..a },
        BottomHalf => Rect { y: a.y + hh, h: a.h - hh, ..a },
        TopLeft => Rect { w: hw, h: hh, ..a },
        TopRight => Rect { x: a.x + hw, w: a.w - hw, h: hh, ..a },
        BottomLeft => Rect { y: a.y + hh, w: hw, h: a.h - hh, ..a },
        BottomRight => Rect { x: a.x + hw, y: a.y + hh, w: a.w - hw, h: a.h - hh },
        FirstThird => Rect { w: third, ..a },
        CenterThird => Rect { x: a.x + third, w: third, ..a },
        // 정수 나눗셈 나머지는 마지막 칸이 흡수해 작업 영역 끝까지 채운다
        LastThird => Rect { x: a.x + third * 2, w: a.w - third * 2, ..a },
        FirstTwoThirds => Rect { w: third * 2, ..a },
        LastTwoThirds => Rect { x: a.x + third, w: a.w - third, ..a },
        Maximize => a,
        Center => {
            let w = current.w.min(a.w);
            let h = current.h.min(a.h);
            Rect { x: a.x + (a.w - w) / 2, y: a.y + (a.h - h) / 2, w, h }
        }
        NextDisplay => current, // 모니터 이동은 next_display 가 담당
    }
}

/// 원래 작업 영역 안의 상대 위치·크기 비율을 다음 모니터 작업 영역에 그대로 옮긴다.
pub fn next_display(cur: Rect, from: Rect, to: Rect) -> Rect {
    let sx = |v: i32| (v as i64 * to.w as i64 / from.w as i64) as i32;
    let sy = |v: i32| (v as i64 * to.h as i64 / from.h as i64) as i32;
    Rect { x: to.x + sx(cur.x - from.x), y: to.y + sy(cur.y - from.y), w: sx(cur.w), h: sy(cur.h) }
}

/// 포그라운드 창에 명령을 적용한다. 모니터 순서는 작업 영역 x, y 오름차순.
pub fn apply(cmd: LayoutCommand) -> anyhow::Result<()> {
    use crate::platform::current::window_ops as ops;
    let hwnd = ops::foreground()?;
    let current = ops::frame(hwnd)?;
    let here = ops::monitor_work(hwnd)?;
    let target = if cmd == LayoutCommand::NextDisplay {
        let mut all = ops::monitors()?;
        all.sort_by_key(|r| (r.x, r.y));
        if all.len() < 2 {
            return Ok(());
        }
        let idx = all.iter().position(|r| *r == here).unwrap_or(0);
        next_display(current, here, all[(idx + 1) % all.len()])
    } else {
        compute(cmd, here, current)
    };
    ops::set_frame(hwnd, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect { x: 0, y: 40, w: 1001, h: 700 };

    #[test]
    fn thirds_cover_whole_width() {
        let a = compute(LayoutCommand::FirstThird, WORK, WORK);
        let b = compute(LayoutCommand::CenterThird, WORK, WORK);
        let c = compute(LayoutCommand::LastThird, WORK, WORK);
        assert_eq!((a.x, a.w, b.x, b.w, c.x, c.w), (0, 333, 333, 333, 666, 335));
        assert_eq!(c.x + c.w, WORK.x + WORK.w);
    }

    #[test]
    fn right_half_starts_after_left_half() {
        let l = compute(LayoutCommand::LeftHalf, WORK, WORK);
        let r = compute(LayoutCommand::RightHalf, WORK, WORK);
        assert_eq!(l.x + l.w, r.x);
        assert_eq!(r.x + r.w, 1001);
        assert_eq!(r.y, 40);
    }

    #[test]
    fn center_clamps_oversized_window() {
        let cur = Rect { x: -50, y: 0, w: 3000, h: 300 };
        let c = compute(LayoutCommand::Center, WORK, cur);
        assert_eq!(c, Rect { x: 0, y: 40 + 200, w: 1001, h: 300 });
    }

    #[test]
    fn next_display_keeps_ratio() {
        let from = Rect { x: 0, y: 0, w: 1920, h: 1040 };
        let to = Rect { x: 1920, y: 0, w: 2560, h: 1400 };
        let cur = Rect { x: 0, y: 0, w: 960, h: 1040 };
        assert_eq!(next_display(cur, from, to), Rect { x: 1920, y: 0, w: 1280, h: 1400 });
    }
}
