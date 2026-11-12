// Where the dashboard window opens, and its place kept in dashboard_window.json
// (docs/specs/dashboard-window.md).

use serde_json::Value;

// A window's place on the screen: top-left corner and size, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

// The saved place when its title bar is on `work` (the work area of the screen nearest
// it), else 1180 x 820 at `scale`, centred on `work`; never bigger than `work`.
pub(super) fn first_rect(saved: Option<Rect>, work: Rect, scale: f32) -> Rect {
    let cap = |r: Rect| Rect { w: r.w.min(work.w), h: r.h.min(work.h), ..r };
    if let Some(s) = saved.filter(|&s| title_on(s, work)) {
        return cap(s);
    }
    let size = |v: f32| (v * scale).round() as i32;
    let (w, h) = (size(1180.0).min(work.w), size(820.0).min(work.h));
    Rect { x: work.x + (work.w - w) / 2, y: work.y + (work.h - h) / 2, w, h }
}

// The window's top strip (its title bar) is on `work`: at least 100 px of it across, and
// its top inside (a little above is the frame's own margin).
fn title_on(r: Rect, work: Rect) -> bool {
    let across = (r.x + r.w).min(work.x + work.w) - r.x.max(work.x);
    across >= 100 && r.y >= work.y - 16 && r.y <= work.y + work.h - 30
}

pub(super) fn rect_json(r: Rect) -> Value {
    serde_json::json!({"x": r.x, "y": r.y, "w": r.w, "h": r.h})
}

// None unless all four numbers are there and the size is not empty.
pub(super) fn rect_from_json(v: &Value) -> Option<Rect> {
    let n = |k: &str| v[k].as_i64().map(|n| n as i32);
    let r = Rect { x: n("x")?, y: n("y")?, w: n("w")?, h: n("h")? };
    (r.w > 0 && r.h > 0).then_some(r)
}

#[cfg(test)]
#[path = "dash_rect_tests.rs"]
mod tests;
