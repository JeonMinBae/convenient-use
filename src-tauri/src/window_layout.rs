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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Left, Right }

/// 순환 단계: 1/2 → 1/3 → 3/4. 오른쪽 폭은 기존 RightHalf·LastThird 와 같은 식(나머지 흡수)으로 맞춘다.
fn half_steps(side: Side, a: Rect) -> [Rect; 3] {
    let widths = match side {
        Side::Left => [a.w / 2, a.w / 3, a.w * 3 / 4],
        Side::Right => [a.w - a.w / 2, a.w - (a.w / 3) * 2, a.w - a.w / 4],
    };
    widths.map(|w| match side {
        Side::Left => Rect { w, ..a },
        Side::Right => Rect { x: a.x + a.w - w, w, ..a },
    })
}

/// 최소 크기 제한이 있는 창은 요청과 몇 px 어긋나므로 허용 오차를 둔다
fn matches(a: Rect, b: Rect) -> bool {
    const TOL: i32 = 2;
    (a.x - b.x).abs() <= TOL && (a.y - b.y).abs() <= TOL && (a.w - b.w).abs() <= TOL && (a.h - b.h).abs() <= TOL
}

/// side 방향으로 바로 붙은 모니터. 세로 범위가 겹치는 것 중 가장 가까운 것.
fn neighbor(side: Side, here: Rect, all: &[Rect]) -> Option<Rect> {
    let overlaps = |r: &Rect| r.y < here.y + here.h && here.y < r.y + r.h;
    all.iter().copied().filter(|r| *r != here && overlaps(r)).filter_map(|r| {
        let gap = match side {
            Side::Left if r.x + r.w <= here.x => here.x - (r.x + r.w),
            Side::Right if r.x >= here.x + here.w => r.x - (here.x + here.w),
            _ => return None,
        };
        Some((gap, r))
    }).min_by_key(|(gap, _)| *gap).map(|(_, r)| r)
}

pub fn cycle_half(side: Side, current: Rect, here: Rect, all: &[Rect]) -> Rect {
    let steps = half_steps(side, here);
    match steps.iter().position(|s| matches(*s, current)) {
        Some(i) if i + 1 < steps.len() => steps[i + 1],
        // 마지막 단계 다음: 그 방향 모니터의 진입하는 쪽 1/2 (왼쪽으로 가면 그 모니터의 오른쪽 반)
        Some(_) => match neighbor(side, here, all) {
            Some(n) => half_steps(if side == Side::Left { Side::Right } else { Side::Left }, n)[0],
            None => steps[0],
        },
        None => steps[0],
    }
}

/// 포그라운드 창에 명령을 적용한다. 모니터 순서는 작업 영역 x, y 오름차순.
pub fn apply(cmd: LayoutCommand) -> anyhow::Result<()> {
    use crate::platform::current::window_ops as ops;
    let hwnd = ops::foreground()?;
    let current = ops::frame(hwnd)?;
    let here = ops::monitor_work(hwnd)?;
    let target = match cmd {
        LayoutCommand::LeftHalf | LayoutCommand::RightHalf => {
            let side = if cmd == LayoutCommand::LeftHalf { Side::Left } else { Side::Right };
            let all = ops::monitors().unwrap_or_else(|_| vec![here]);
            cycle_half(side, current, here, &all)
        }
        LayoutCommand::NextDisplay => {
            let mut all = ops::monitors()?;
            all.sort_by_key(|r| (r.x, r.y));
            if all.len() < 2 {
                return Ok(());
            }
            let idx = all.iter().position(|r| *r == here).unwrap_or(0);
            next_display(current, here, all[(idx + 1) % all.len()])
        }
        _ => compute(cmd, here, current),
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

    const MAIN: Rect = Rect { x: 0, y: 0, w: 1920, h: 1032 };
    const LEFT_MON: Rect = Rect { x: -1920, y: 0, w: 1920, h: 1032 };

    #[test]
    fn left_cycles_half_third_three_quarters_then_moves_left() {
        let all = [MAIN, LEFT_MON];
        let free = Rect { x: 300, y: 200, w: 800, h: 600 };
        let s1 = cycle_half(Side::Left, free, MAIN, &all);
        assert_eq!(s1, Rect { x: 0, y: 0, w: 960, h: 1032 });
        let s2 = cycle_half(Side::Left, s1, MAIN, &all);
        assert_eq!(s2, Rect { x: 0, y: 0, w: 640, h: 1032 });
        let s3 = cycle_half(Side::Left, s2, MAIN, &all);
        assert_eq!(s3, Rect { x: 0, y: 0, w: 1440, h: 1032 });
        let s4 = cycle_half(Side::Left, s3, MAIN, &all);
        assert_eq!(s4, Rect { x: -960, y: 0, w: 960, h: 1032 }); // 왼쪽 모니터의 오른쪽 반
        // 넘어간 모니터에서 다시 누르면 그 모니터의 왼쪽 반부터
        assert_eq!(cycle_half(Side::Left, s4, LEFT_MON, &all), Rect { x: -1920, y: 0, w: 960, h: 1032 });
    }

    #[test]
    fn last_step_without_neighbor_returns_to_half() {
        let s3 = Rect { x: -1920, y: 0, w: 1440, h: 1032 };
        assert_eq!(cycle_half(Side::Left, s3, LEFT_MON, &[MAIN, LEFT_MON]), Rect { x: -1920, y: 0, w: 960, h: 1032 });
        let r3 = Rect { x: 480, y: 0, w: 1440, h: 1032 };
        assert_eq!(cycle_half(Side::Right, r3, MAIN, &[MAIN]), Rect { x: 960, y: 0, w: 960, h: 1032 });
    }

    #[test]
    fn right_steps_end_at_work_area_edge_and_tolerate_min_size() {
        let odd = Rect { x: 0, y: 40, w: 1001, h: 700 };
        let s1 = cycle_half(Side::Right, odd, odd, &[odd]);
        assert_eq!(s1, compute(LayoutCommand::RightHalf, odd, odd)); // 기존 RightHalf 와 같은 1/2
        // 창이 1px 덜 줄어든 경우도 같은 단계로 본다
        let nudged = Rect { x: s1.x - 1, w: s1.w + 1, ..s1 };
        let s2 = cycle_half(Side::Right, nudged, odd, &[odd]);
        assert_eq!(s2, compute(LayoutCommand::LastThird, odd, odd));
        assert_eq!(s2.x + s2.w, 1001);
    }

    #[test]
    fn neighbor_ignores_monitors_without_vertical_overlap() {
        let below_left = Rect { x: -1920, y: 1080, w: 1920, h: 1032 };
        let s3 = Rect { x: 0, y: 0, w: 1440, h: 1032 };
        assert_eq!(cycle_half(Side::Left, s3, MAIN, &[MAIN, below_left]), Rect { x: 0, y: 0, w: 960, h: 1032 });
    }
}
