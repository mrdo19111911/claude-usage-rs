// The switch buttons on each "Switch to" row: where they sit, what a click on one
// does, and drawing them; per window, what the pointer is over (hover, footer).

use super::*;
use std::cell::Cell;
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

fn inside(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

// The two slots, right-aligned on a row whose text starts at `y`: CLI, then Desktop.
// Fixed: a missing button leaves its slot empty, so the columns line up.
fn slots(w: i32, s: f32, y: i32) -> (RECT, RECT) {
    let (top, bottom) = (y, y + px(18.0, s));
    let dr = w - px(14.0, s);
    let dl = dr - px(58.0, s);
    let cr = dl - px(4.0, s);
    let cl = cr - px(34.0, s);
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
    buttons.iter().find(|(r, _)| inside(r, x, y)).map(|(_, k)| k.clone())
}

// What each window keeps between paints: its buttons and footer as last drawn, and
// where the pointer is (None: outside).
#[derive(Default)]
struct Win {
    hits: Vec<(RECT, Pick)>,
    footer: Option<RECT>,
    pointer: Option<(i32, i32)>,
}

thread_local! {
    // Per window: [hover, pinned].
    static WINS: RefCell<[Win; 2]> = RefCell::new([Win::default(), Win::default()]);
    // The window being drawn now (see begin_draw).
    static DRAWING: Cell<bool> = const { Cell::new(false) };
}

fn win<R>(pinned: bool, f: impl FnOnce(&mut Win) -> R) -> R {
    WINS.with(|w| f(&mut w.borrow_mut()[pinned as usize]))
}

pub(super) fn set_hits(pinned: bool, b: Vec<(RECT, Pick)>) {
    win(pinned, |w| w.hits = b);
}

pub(super) fn hit_at(pinned: bool, x: i32, y: i32) -> Option<Pick> {
    win(pinned, |w| hit(&w.hits, x, y))
}

// The footer's "Open dashboard" link, per window, as last drawn (layout.rs sets it).
pub(super) fn set_footer(pinned: bool, r: RECT) {
    win(pinned, |w| w.footer = Some(r));
}

pub(super) fn in_footer(pinned: bool, x: i32, y: i32) -> bool {
    win(pinned, |w| w.footer.is_some_and(|r| inside(&r, x, y)))
}

// widget.rs keeps the pointer's place: WM_MOUSEMOVE sets it, WM_MOUSELEAVE clears it.
pub(super) fn set_pointer(pinned: bool, at: Option<(i32, i32)>) {
    win(pinned, |w| w.pointer = at);
}

pub(super) fn pointer(pinned: bool) -> Option<(i32, i32)> {
    win(pinned, |w| w.pointer)
}

// The pointer is over `r` in that widget (hover state: filled instead of outlined).
pub(super) fn hot(pinned: bool, r: &RECT) -> bool {
    pointer(pinned).is_some_and(|(x, y)| inside(r, x, y))
}

// What a point is over: the widget repaints only when this changes, and shows a hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Spot {
    None,
    Head,
    Footer,
    Button(usize),
}

pub(super) fn spot(pinned: bool, x: i32, y: i32, head: &RECT) -> Spot {
    if inside(head, x, y) {
        return Spot::Head;
    }
    if in_footer(pinned, x, y) {
        return Spot::Footer;
    }
    win(pinned, |w| w.hits.iter().position(|(r, _)| inside(r, x, y))).map_or(Spot::None, Spot::Button)
}

// Which window draw_buttons draws for: widget.rs paint() calls this before layout(),
// so the drawing code keeps its `draw_buttons(&p, &b)` call.
pub(super) fn begin_draw(pinned: bool) {
    DRAWING.with(|d| d.set(pinned));
}

pub(super) fn drawing() -> bool {
    DRAWING.with(|d| d.get())
}

// Outlined in the chip colour; filled, label on BG, when the pointer is over it.
pub(super) unsafe fn draw_buttons(p: &Painter, b: &[(RECT, Pick)]) {
    let pinned = drawing();
    for (r, k) in b {
        let (label, c) = match k {
            Pick::Cli(_) => ("CLI", CODE),
            Pick::Desktop { .. } => ("Desktop", DESK),
        };
        let fg = if hot(pinned, r) {
            p.round(r.left, r.top, r.right, r.bottom, p.px(5.0), c);
            BG
        } else {
            p.outline(r.left, r.top, r.right, r.bottom, p.px(5.0), c);
            c
        };
        p.text(r.left + (r.right - r.left - p.width_of(label, SMALL)) / 2, r.top + p.px(1.5), label, fg, SMALL);
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
