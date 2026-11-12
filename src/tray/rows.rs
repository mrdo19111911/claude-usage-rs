// Account rows that open (docs/specs/gadget-chart.md): which row is open (one at a time,
// both widgets, not kept across restarts), where each row is as last drawn, and the
// window growing or shrinking to fit when one opens or closes.

use super::*;

thread_local! {
    static OPEN: RefCell<Option<String>> = const { RefCell::new(None) };
    // Per window: [hover, pinned]; (row rectangle, account id).
    static ROWS: RefCell<[Vec<(RECT, String)>; 2]> = RefCell::new([vec![], vec![]]);
}

// After a click on row `id` with `open` open: a second click closes it, another row
// takes its place.
pub(super) fn toggled(open: Option<&str>, id: &str) -> Option<String> {
    (open != Some(id)).then(|| id.to_string())
}

pub(super) fn open_row() -> Option<String> {
    OPEN.with(|o| o.borrow().clone())
}

pub(super) fn is_open(id: &str) -> bool {
    OPEN.with(|o| o.borrow().as_deref() == Some(id))
}

pub(super) fn set_open(v: Option<String>) {
    OPEN.with(|o| *o.borrow_mut() = v);
}

// layout() clears a window's rows before drawing; each section adds its own.
pub(super) fn clear_rows(pinned: bool) {
    ROWS.with(|r| r.borrow_mut()[pinned as usize].clear());
}

pub(super) fn add_row(pinned: bool, r: RECT, id: &str) {
    ROWS.with(|v| v.borrow_mut()[pinned as usize].push((r, id.to_string())));
}

pub(super) fn row_at(pinned: bool, x: i32, y: i32) -> Option<String> {
    ROWS.with(|v| v.borrow()[pinned as usize].iter().find(|(r, _)| x >= r.left && x < r.right && y >= r.top && y < r.bottom).map(|(_, id)| id.clone()))
}

// What takes a click (a hand, and no dragging the pinned widget): buttons, rows, footer.
pub(super) fn clickable(pinned: bool, x: i32, y: i32, head: &RECT) -> bool {
    spot(pinned, x, y, head) != Spot::None || row_at(pinned, x, y).is_some()
}

// Where a window of height `h` goes so it stays in the work area: its top unless the
// bottom would leave the area, then moved up (the hover widget sits above the tray).
pub(super) fn grown_top(top: i32, h: i32, wa: &RECT) -> i32 {
    if top + h > wa.bottom { (wa.bottom - h).max(wa.top) } else { top }
}

// A click on row `id`: opens or closes it, then both widgets fit what they show.
pub(super) unsafe fn toggle_row(id: &str) {
    set_open(toggled(open_row().as_deref(), id));
    let (hover, pinned) = ui(|u| (u.hover, u.pinned));
    for (w, p) in [(hover, false), (pinned, true)] {
        if IsWindowVisible(w) == 0 {
            continue;
        }
        let s = GetDpiForWindow(w) as f32 / 96.0;
        let h = layout(&ui(|u| u.model.clone()), s, p, ptr::null_mut());
        let mut r: RECT = mem::zeroed();
        GetWindowRect(w, &mut r);
        let top = grown_top(r.top, h, &work_area(r.left, r.top));
        SetWindowPos(w, HWND_TOPMOST, r.left, top, width(s), h, SWP_NOACTIVATE);
        InvalidateRect(w, ptr::null(), FALSE);
    }
}
