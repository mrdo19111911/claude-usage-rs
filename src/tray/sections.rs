// The widget's sections (docs/specs/modules.md, "Widget v2"): the in-use card, the
// "Switch to" rows, the "Used up · available at" rows. layout.rs puts them in order.

use super::*;

// Claude Code's chip and button label (docs/REVIEW_DE_HIEU.md, "Names to use everywhere").
pub(super) const CODE_LABEL: &str = "Code";
use super::words::{available_at, figures, resets, FIGS_KEY, IN_USE, NOT_CHECKED, NO_USE, SWITCH_TO, USED_UP};

const CARD: u32 = rgb(0x23, 0x2a, 0x31);
// The pace tick on a card's bars.
const TICK: u32 = rgb(0xd8, 0xd8, 0xdc);

// The name part of a full email ("bob" for "bob@x.com"); anything else as it is.
pub(super) fn short(e: &str) -> &str {
    match e.split_once('@') {
        Some((n, d)) if !n.is_empty() && d.contains('.') => n,
        _ => e,
    }
}

// IN_USE, then the in-use card(s): one for both sides on one account, else one per side
// (none for a missing Claude Desktop). Returns the y under them.
pub(super) unsafe fn in_use(p: &Painter, m: &Model, w: i32, y: i32, draw: bool) -> i32 {
    let y = title(p, IN_USE, y, draw);
    let gap = p.px(8.0);
    if let (true, Some(d)) = (same_account(m), &m.desk) {
        let both = Side { email: m.cli.email.clone().or(d.email.clone()), usage: m.cli.usage.clone().or(d.usage.clone()) };
        return card(p, m, &both, &[(CODE_LABEL, CODE), ("Desktop", DESK)], w, y, draw) + gap;
    }
    let mut y = card(p, m, &m.cli, &[(CODE_LABEL, CODE)], w, y, draw) + gap;
    if let Some(d) = &m.desk {
        y = card(p, m, d, &[("Desktop", DESK)], w, y, draw) + gap;
    }
    y
}

// One card: dot, name, chips; then each window's figures and bar. Returns its bottom.
unsafe fn card(p: &Painter, m: &Model, sd: &Side, chips: &[(&str, u32)], w: i32, y: i32, draw: bool) -> i32 {
    let (l, r) = (p.px(14.0), w - p.px(14.0));
    let (cl, cr) = (l + p.px(10.0), r - p.px(10.0));
    let body = match (&sd.usage, &sd.email) {
        (Some(_), _) => 2.0 * 36.0,
        (None, Some(_)) => 26.0,
        (None, None) => 0.0,
    };
    let bottom = y + p.px(40.0 + body);
    if !draw {
        return bottom;
    }
    p.round(l, y, r, bottom, p.px(8.0), CARD);
    // Header line: dot, name, chips at the right.
    let cy = y + p.px(20.0);
    let dot = if sd.email.is_some() { GREEN } else { MUTED };
    p.round(cl, cy - p.px(3.0), cl + p.px(6.0), cy + p.px(3.0), p.px(3.0), dot);
    let mut x = cr;
    for (label, c) in chips.iter().rev() {
        let cw = p.width_of(label, SMALL) + p.px(12.0);
        let chip = RECT { left: x - cw, top: cy - p.px(8.0), right: x, bottom: cy + p.px(8.0) };
        p.outline(chip.left, chip.top, chip.right, chip.bottom, p.px(8.0), *c);
        p.centred(&chip, label, *c, SMALL);
        x -= cw + p.px(4.0);
    }
    let nx = cl + p.px(12.0);
    let (name, c) = sd.email.as_deref().map_or(("Not signed in", MUTED), |e| (short(e), TEXT));
    p.text_mid(nx, cy, &p.fitted(name, NORMAL, x - p.px(4.0) - nx), c, NORMAL);
    let mut yy = y + p.px(30.0);
    let Some(u) = &sd.usage else {
        if sd.email.is_some() {
            p.text_mid(cl, yy + p.px(17.0), NOT_CHECKED, MUTED, NORMAL);
        }
        return bottom;
    };
    for (label, win, len) in [("5-hour", &u.five, FIVE_MS), ("Weekly", &u.week, WEEK_MS)] {
        let cy = yy + p.px(17.0);
        p.text_mid(cl, cy, label, MUTED, NORMAL);
        p.text_mid(cl + p.px(54.0), cy, &format!("{}%", win.u), TEXT, BOLD);
        if let Some(t) = win.reset.filter(|&t| t > m.now) {
            p.text_mid_right(cr, cy, &resets(t, m.now), MUTED, SMALL);
        } else if win.reset.is_none() {
            p.text_mid_right(cr, cy, NO_USE, MUTED, SMALL);
        }
        let bt = yy + p.px(30.0);
        p.bar(cl, bt, cr, bt + p.px(6.0), win.u);
        if let Some(f) = pace(win.reset, m.now, len) {
            let tx = cl + ((cr - cl) as f32 * f.clamp(0.0, 1.0)).round() as i32 - p.px(1.0);
            p.fill(tx, bt - p.px(2.0), tx + p.px(2.0).max(1), bt + p.px(8.0), TICK);
        }
        yy += p.px(36.0);
    }
    bottom
}

// A section's small caps title from `y`; returns the y under it.
unsafe fn title(p: &Painter, s: &str, y: i32, draw: bool) -> i32 {
    if draw {
        p.text_mid(p.px(14.0), y + p.px(13.0), s, MUTED, SMALL);
    }
    y + p.px(22.0)
}

// SWITCH_TO: name, 5-hour and weekly bars, figures, the Code / Desktop buttons. Sets the
// widget's buttons (none when the list is empty). Returns the y under the rows.
pub(super) unsafe fn switch_to(p: &Painter, m: &Model, list: &[&Other], pinned: bool, w: i32, y: i32, draw: bool) -> i32 {
    let mut all = vec![];
    let (l, tr) = (p.px(14.0), text_right(w, p.s));
    if draw && !list.is_empty() {
        p.text_mid_right(tr, y + p.px(13.0), FIGS_KEY, MUTED, SMALL);
    }
    let mut y = if list.is_empty() { y } else { title(p, SWITCH_TO, y, draw) };
    let (br, bl) = (tr - p.px(66.0), tr - p.px(66.0 + 40.0)); // figures up to "100% · 100%"
    for o in list {
        let b = row_buttons(m, o, w, p.s, y + p.px(3.0));
        if draw {
            let cy = y + p.px(12.0);
            let name = p.fitted(short(&o.email), NORMAL, bl - p.px(8.0) - l);
            p.text_mid(l, cy, &name, TEXT, NORMAL);
            let (five, week) = o.usage.as_ref().map_or((0, 0), |u| (u.five.u, u.week.u));
            p.bar(bl, cy - p.px(5.0), br, cy - p.px(1.0), five);
            p.bar(bl, cy + p.px(1.0), br, cy + p.px(5.0), week);
            p.text_mid_right(tr, cy, &figures(o.usage.as_ref()), MUTED, SMALL);
            draw_buttons(p, &b);
        }
        all.extend(b);
        y = row_end(p, m, o, pinned, w, y, p.px(24.0), draw);
    }
    if draw {
        // Where the widget finds the buttons on a click.
        set_hits(pinned, all);
    }
    y
}

// USED_UP: muted name and when the account can be used again.
pub(super) unsafe fn full_rows(p: &Painter, m: &Model, list: &[(&Other, i64)], pinned: bool, w: i32, y: i32, draw: bool) -> i32 {
    let mut y = if list.is_empty() { y } else { title(p, USED_UP, y, draw) };
    let (l, r) = (p.px(14.0), w - p.px(14.0));
    for (o, t) in list {
        if draw {
            let cy = y + p.px(11.0);
            let at = available_at(*t, m.now);
            let aw = p.width_of(&at, SMALL);
            p.text_mid(l, cy, &p.fitted(short(&o.email), NORMAL, r - aw - p.px(8.0) - l), MUTED, NORMAL);
            p.text_mid_right(r, cy, &at, MUTED, SMALL);
        }
        y = row_end(p, m, o, pinned, w, y, p.px(22.0), draw);
    }
    y
}

// Under a row `h` tall from `y`: a click on it opens its chart (rows.rs), drawn here when
// open. Returns the y under both.
unsafe fn row_end(p: &Painter, m: &Model, o: &Other, pinned: bool, w: i32, y: i32, h: i32, draw: bool) -> i32 {
    if draw {
        add_row(pinned, RECT { left: 0, top: y, right: w, bottom: y + h }, &o.id);
    }
    if is_open(&o.id) { row_chart(p, m, o, w, y + h, draw) } else { y + h }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The review (docs/REVIEW_DE_HIEU.md): "Code", never "CLI". Figures: words.rs.
    #[test]
    fn rows_say_code() {
        assert_eq!(CODE_LABEL, "Code");
    }

    #[test]
    fn short_drops_the_domain_of_a_full_email_only() {
        assert_eq!(short("bob@example.com"), "bob");
        assert_eq!(short("Personal"), "Personal");
        assert_eq!(short("@x.com"), "@x.com");
        assert_eq!(short("a@localhost"), "a@localhost");
    }
}
