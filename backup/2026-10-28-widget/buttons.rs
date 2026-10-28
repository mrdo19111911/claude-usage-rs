// The switch buttons on each "Other accounts" row: where they sit, what a click on one
// does, and drawing them.

use super::*;
use std::sync::atomic::AtomicBool;

// What a row button asks for; becomes an `Act` at click time (Act is not Clone).
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Pick {
    Cli(String),
    // The email is for the Yes/No box.
    Desktop { uuid: String, email: String },
}

impl Pick {
    pub(super) fn act(self) -> Act {
        match self {
            Pick::Cli(id) => Act::Cli(id),
            Pick::Desktop { uuid, .. } => Act::Desktop(uuid),
        }
    }
}

fn px(v: f32, s: f32) -> i32 {
    (v * s).round() as i32
}

// The two slots, right-aligned on a row whose text starts at `y`: CLI, then Desktop.
fn slots(w: i32, s: f32, y: i32) -> (RECT, RECT) {
    let (top, bottom) = (y, y + px(18.0, s));
    let dr = w - px(14.0, s);
    let dl = dr - px(54.0, s);
    let cr = dl - px(4.0, s);
    let cl = cr - px(32.0, s);
    (RECT { left: cl, top, right: cr, bottom }, RECT { left: dl, top, right: dr, bottom })
}

// Where the row's other text must end: left of the slots, present or not, so the
// figures line up from row to row.
pub(super) fn text_right(w: i32, s: f32) -> i32 {
    slots(w, s, 0).0.left - px(8.0, s)
}

// The buttons of one row, with what each does.
pub(super) fn row_buttons(m: &Model, o: &Other, w: i32, s: f32, y: i32) -> Vec<(RECT, Pick)> {
    let (cli, desk) = slots(w, s, y);
    let mut v = vec![];
    if !o.in_cli {
        v.push((cli, Pick::Cli(o.id.clone())));
    }
    if let Some(uuid) = o.desk_uuid.as_ref().filter(|_| !o.in_desk && m.desk.is_some() && !m.desk_switching) {
        v.push((desk, Pick::Desktop { uuid: uuid.clone(), email: o.email.clone() }));
    }
    v
}

pub(super) fn hit(buttons: &[(RECT, Pick)], x: i32, y: i32) -> Option<Pick> {
    buttons.iter().find(|(r, _)| x >= r.left && x < r.right && y >= r.top && y < r.bottom).map(|(_, k)| k.clone())
}

thread_local! {
    // The buttons last drawn, per window: [hover, pinned].
    static HITS: RefCell<[Vec<(RECT, Pick)>; 2]> = RefCell::new([vec![], vec![]]);
}

pub(super) fn set_hits(pinned: bool, b: Vec<(RECT, Pick)>) {
    HITS.with(|h| h.borrow_mut()[pinned as usize] = b);
}

pub(super) fn hit_at(pinned: bool, x: i32, y: i32) -> Option<Pick> {
    HITS.with(|h| hit(&h.borrow()[pinned as usize], x, y))
}

pub(super) unsafe fn draw_buttons(p: &Painter, b: &[(RECT, Pick)]) {
    for (r, k) in b {
        let (label, c) = match k {
            Pick::Cli(_) => ("CLI", CODE),
            Pick::Desktop { .. } => ("Desktop", DESK),
        };
        p.fill(r.left, r.top, r.right, r.bottom, c);
        p.text(r.left + (r.right - r.left - p.width_of(label, SMALL)) / 2, r.top + p.px(1.5), label, BG, SMALL);
    }
}

// One switch at a time.
static BUSY: AtomicBool = AtomicBool::new(false);

// A click on a row button: Desktop asks first; the switch runs off the tray thread, a
// failure shows in a box, then the tray refreshes (TIMER_REFRESH, as msg_proc handles it).
pub(super) unsafe fn switch(hwnd: HWND, k: Pick) {
    if BUSY.load(Ordering::SeqCst) {
        return;
    }
    if let Pick::Desktop { email, .. } = &k {
        let q = format!("Claude Desktop will close and open again on {email}. Running tasks stop. Go on?");
        let style = MB_YESNO | MB_ICONWARNING | MB_TOPMOST | MB_SETFOREGROUND;
        if MessageBoxW(hwnd, wide(&q).as_ptr(), wide("Switch Claude Desktop").as_ptr(), style) != IDYES {
            return;
        }
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let (act, msg) = ui(|u| (u.act.clone(), u.msg as usize));
    thread::spawn(move || {
        let r = act(k.act());
        BUSY.store(false, Ordering::SeqCst);
        if let Err(e) = r {
            MessageBoxW(ptr::null_mut(), wide(&e).as_ptr(), wide("Claude usage").as_ptr(), MB_ICONERROR | MB_TOPMOST | MB_SETFOREGROUND);
        }
        PostMessageW(msg as HWND, WM_TIMER, TIMER_REFRESH, 0);
    });
}

#[cfg(test)]
#[path = "buttons_tests.rs"]
mod tests;
