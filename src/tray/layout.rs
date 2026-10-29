// The widget's layout: what goes where, top to bottom (docs/specs/modules.md, "Widget v2").
// The sections themselves: sections.rs.

use super::*;

// How old the newest figures are, for the header: "· just now", "· 3 min ago", "· 2 h ago".
pub(super) fn ago(checked: i64, now: i64) -> String {
    match (now - checked).max(0) / 60_000 {
        0 => "· just now".into(),
        m @ 1..=59 => format!("· {m} min ago"),
        m @ 60..=1439 => format!("· {} h ago", m / 60),
        m => format!("· {} d ago", m / 1440),
    }
}

// The widget's fonts at scale `s` (NORMAL, BOLD, SMALL, ICON); none without a DC.
unsafe fn fonts(s: f32, dc: HDC) -> [HFONT; 4] {
    let font = |h: f32, weight: i32, face: &str| {
        if dc.is_null() {
            return ptr::null_mut();
        }
        CreateFontW(-(h * s).round() as i32, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, wide(face).as_ptr())
    };
    [font(13.0, 400, "Segoe UI"), font(13.0, 600, "Segoe UI"), font(11.0, 400, "Segoe UI"), font(12.0, 400, "Segoe MDL2 Assets")]
}

// Draws the widget when `dc` is set; returns its height either way (the same).
pub(super) unsafe fn layout(m: &Model, s: f32, pinned: bool, dc: HDC) -> i32 {
    let w = width(s);
    let p = Painter { dc, s, fonts: fonts(s, dc) };
    let draw = !dc.is_null();
    // The DC's own font, put back before ours are deleted.
    let old = if draw { SelectObject(dc, p.fonts[NORMAL]) } else { ptr::null_mut() };
    if draw {
        SetBkMode(dc, TRANSPARENT as i32);
        p.fill(0, 0, w, 10_000, BG);
    }
    let mut y = header(&p, m, pinned, w, draw);
    y = in_use(&p, m, w, y, draw);
    let (free, full) = split_others(m);
    y = switch_to(&p, m, &free, pinned, w, y, draw);
    y = full_rows(&p, m, &full, w, y, draw);
    y = footer(&p, pinned, w, y, draw);
    if draw {
        // Border last, over everything, along the window's rounded corners.
        p.outline(0, 0, w, y, p.px(10.0), BORDER);
        SelectObject(dc, old);
        for f in p.fonts {
            DeleteObject(f);
        }
    }
    y
}

// "Claude usage · 3 min ago" and the pin button (widget::button). Returns the y under it.
unsafe fn header(p: &Painter, m: &Model, pinned: bool, w: i32, draw: bool) -> i32 {
    let b = button(p.s, w);
    if draw {
        let (pad, cy) = (p.px(14.0), (b.top + b.bottom) / 2);
        p.text_mid(pad, cy, "Claude usage", TEXT, BOLD);
        if let Some(t) = m.checked {
            p.text_mid(pad + p.width_of("Claude usage", BOLD) + p.px(5.0), cy, &ago(t, m.now), MUTED, NORMAL);
        }
        let on = hot(pinned, &b);
        if on {
            p.round(b.left, b.top, b.right, b.bottom, p.px(6.0), TRACK);
        }
        let glyph = if pinned { "\u{E77A}" } else { "\u{E718}" };
        p.centred(&b, glyph, if on { TEXT } else { MUTED }, ICON);
    }
    b.bottom + p.px(8.0)
}

// A line above, then "Open dashboard" and the external-link glyph; the whole line is the
// link (buttons::set_footer). Returns the widget's bottom.
unsafe fn footer(p: &Painter, pinned: bool, w: i32, y: i32, draw: bool) -> i32 {
    let y = y + p.px(4.0);
    let r = RECT { left: 0, top: y + 1, right: w, bottom: y + 1 + p.px(32.0) };
    if draw {
        p.fill(p.px(1.0).max(1), y, w - p.px(1.0).max(1), y + 1, BORDER);
        let (pad, cy) = (p.px(14.0), (r.top + r.bottom) / 2);
        let c = if hot(pinned, &r) { TEXT } else { MUTED };
        p.text_mid(pad, cy, "Open dashboard", c, NORMAL);
        p.text_mid(pad + p.width_of("Open dashboard", NORMAL) + p.px(6.0), cy, "\u{E8A7}", c, ICON);
        set_footer(pinned, r);
    }
    r.bottom
}
