// The chart under an open account row (docs/specs/gadget-chart.md): 5-hour on the left,
// weekly on the right. Points: chart_data.rs; smooth lines: gdip.rs; text: GDI.

use super::*;

const FIVE_C: u32 = rgb(0x4c, 0x8d, 0xf6);
const WEEK_C: u32 = rgb(0xf0, 0x65, 0x3c);
// Grid lines: visible on the card (0x232a31) without competing with the data.
const GRID: u32 = rgb(0x3d, 0x46, 0x50);
// The axes (Ox at 0%, Oy at the left), lighter than the grid.
const AXIS: u32 = rgb(0x6a, 0x72, 0x7c);

// The chart's height at scale `s`.
pub(super) fn chart_h(s: f32) -> i32 {
    (100.0 * s).round() as i32
}

// One window's panel: what it shows, taken from the row's account.
struct Panel<'a> {
    label: &'a str,
    week: bool,
    len: i64,
    line: i64,
    colour: u32,
}

// Draws `o`'s chart from `y` across the widget's width `w` (nothing without `draw`);
// returns the y under it.
pub(super) unsafe fn row_chart(p: &Painter, m: &Model, o: &Other, w: i32, y: i32, draw: bool) -> i32 {
    let bottom = y + chart_h(p.s);
    if !draw {
        return bottom;
    }
    p.round(p.px(10.0), y, w - p.px(10.0), bottom - p.px(4.0), p.px(8.0), rgb(0x23, 0x2a, 0x31));
    let (l, r, gap) = (p.px(20.0), w - p.px(20.0), p.px(14.0));
    let mid = (l + r) / 2;
    let five = Panel { label: "5-hour", week: false, len: FIVE_MS, line: m.five_line, colour: FIVE_C };
    let week = Panel { label: "Weekly", week: true, len: WEEK_MS, line: m.week_line, colour: WEEK_C };
    panel(p, m, o, &five, l, mid - gap / 2, y);
    panel(p, m, o, &week, mid + gap / 2, r, y);
    bottom
}

unsafe fn panel(p: &Painter, m: &Model, o: &Other, k: &Panel, l: i32, r: i32, y: i32) {
    let win = o.usage.as_ref().map(|u| if k.week { &u.week } else { &u.five });
    let (u, reset, was) = win.map_or((None, None, None), |w| {
        let (u, r, was) = eff(w, m.now);
        (Some(u), r, was)
    });
    // Title: "5-hour 35%".
    let ty = y + p.px(14.0);
    p.text_mid(l, ty, k.label, MUTED, SMALL);
    if let Some(u) = u {
        p.text_mid(l + p.width_of(k.label, SMALL) + p.px(5.0), ty, &format!("{u}%"), TEXT, SMALL);
    }
    let (top, btm) = (y + p.px(26.0), y + p.px(74.0));
    // "100%" and "0%" left of the Oy axis, right-aligned to it; the plot starts there.
    let (l0, l) = (l, l + p.width_of("100%", SMALL) + p.px(4.0));
    for (t, ty) in [("100%", top), ("0%", btm)] {
        p.text_mid(l - p.px(3.0) - p.width_of(t, SMALL), ty, t, MUTED, SMALL);
    }
    let (start, end) = span(reset, m.now, k.len);
    let pts = points(&o.history, k.week, start, m.now, u, was);
    for v in [0, 50, 100] {
        let gy = btm - (btm - top) * v / 100;
        p.fill(l, gy, r, gy + 1, GRID);
    }
    let ly = y + p.px(86.0);
    // Oy at the plot's left, Ox along 0%.
    p.fill(l, top, l + 1, btm + 1, AXIS);
    p.fill(l, btm, r, btm + 1, AXIS);
    if pts.is_empty() {
        p.centred(&RECT { left: l, top, right: r, bottom: btm }, words::NOT_CHECKED, MUTED, SMALL);
        return;
    }
    let b = (l as f32, top as f32, r as f32, btm as f32);
    let x_of = |t: i64| to_xy(&[(t, 0)], start, end, b)[0].0;
    // Each whole hour, or each local midnight, as on the dashboard's charts.
    for t in ticks(start, end, k.week, utc_offset(m.now)) {
        let x = x_of(t).round() as i32;
        p.fill(x, top, x + 1, btm + 1, GRID);
    }
    let Some(g) = Gfx::on(p.dc) else { return };
    let th = 1.0f32.max(p.s);
    let line_y = to_xy(&[(start, k.line)], start, end, b)[0].1;
    g.line((b.0, line_y), (b.2, line_y), argb(k.colour, 0x90), th, true);
    // Resets: the one ahead at the right edge, a passed one at its time; times under them.
    let mut marks = vec![];
    if let Some(t) = reset {
        marks.push((x_of(t), clock(t, m.now)));
    }
    if let Some(t) = was.filter(|&t| t > start) {
        marks.push((x_of(t), clock(t, m.now)));
    }
    for (x, _) in &marks {
        g.line((*x, top as f32 - 2.0 * p.s), (*x, b.3), argb(GREEN, 0xff), 1.5 * p.s, false);
    }
    let xy = thin(&to_xy(&pts, start, end, b));
    g.lines(&xy, argb(k.colour, 0xff), 1.5 * p.s);
    if let (Some(&(x, yy)), Some(_)) = (xy.last(), u) {
        g.dot(x, yy, 2.5 * p.s, argb(k.colour, 0xff));
    }
    drop(g);
    for (x, lab) in marks {
        let tw = p.width_of(&lab, SMALL);
        let tx = (x as i32 - tw / 2).clamp(l0, r - tw);
        p.text_mid(tx, ly, &lab, GREEN, SMALL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Drawn at 1x and 2x into a bitmap: the 5-hour line's colour shows up in the left
    // panel, the weekly one's in the right.
    #[test]
    fn draws_both_lines_at_each_scale() {
        let now = 10 * 3_600_000;
        let usage = Use { five: Win { u: 60, reset: Some(now + 3_600_000) }, week: Win { u: 60, reset: Some(now + 86_400_000) } };
        let hist: Vec<(i64, i64, i64)> = (0..50).map(|i| (now - 6 * 86_400_000 + i * 3 * 3_600_000, 60, 60)).collect();
        let o = Other { usage: Some(usage), history: hist, ..Default::default() };
        let m = Model { now, five_line: 95, week_line: 99, ..Default::default() };
        for s in [1.0f32, 2.0] {
            unsafe {
                let (w, h) = (width(s), chart_h(s));
                let dc = CreateCompatibleDC(ptr::null_mut());
                let bmp = CreateBitmap(w, h, 1, 32, ptr::null());
                let old = SelectObject(dc, bmp);
                let p = Painter { dc, s, fonts: [CreateFontW(-(11.0 * s) as i32, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 0, 0, wide("Segoe UI").as_ptr()); 4] };
                assert_eq!(row_chart(&p, &m, &o, w, 0, true), h);
                let has = |c: u32, x0: i32, x1: i32| (x0..x1).any(|x| (0..h).any(|y| GetPixel(dc, x, y) == c));
                assert!(has(FIVE_C, 0, w / 2), "5-hour line at {s}x");
                assert!(has(WEEK_C, w / 2, w), "weekly line at {s}x");
                // Both panels have their axes (Ox, Oy), in the axis colour.
                assert!(has(AXIS, 0, w / 2) && has(AXIS, w / 2, w), "axes at {s}x");
                DeleteObject(p.fonts[0]);
                SelectObject(dc, old);
                DeleteObject(bmp);
                DeleteDC(dc);
            }
        }
    }
}
