// The widget's layout: what goes where, top to bottom (docs/specs/modules.md, "Widget v2").
// The sections themselves: sections.rs.

use super::*;

// How old the newest figures are, for the footer: "updated 3 min ago" (also just now, h, d).
pub(super) fn ago(checked: i64, now: i64) -> String {
    match (now - checked).max(0) / 60_000 {
        0 => "updated just now".into(),
        m @ 1..=59 => format!("updated {m} min ago"),
        m @ 60..=1439 => format!("updated {} h ago", m / 60),
        m => format!("updated {} d ago", m / 1440),
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
    if draw {
        clear_rows(pinned);
    }
    let mut y = header(&p, pinned, w, draw);
    y = in_use(&p, m, w, y, draw);
    let (free, full) = split_others(m);
    y = switch_to(&p, m, &free, pinned, w, y, draw);
    y = full_rows(&p, m, &full, pinned, w, y, draw);
    y = footer(&p, m, pinned, w, y, draw);
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

// The header's title: the app's name.
const TITLE: &str = words::APP;

// The app's name and the pin button (widget::button); how old the figures are is in the
// footer, as the name leaves no room. Returns the y under it.
unsafe fn header(p: &Painter, pinned: bool, w: i32, draw: bool) -> i32 {
    let b = button(p.s, w);
    if draw {
        let (pad, cy) = (p.px(14.0), (b.top + b.bottom) / 2);
        p.text_mid(pad, cy, TITLE, TEXT, BOLD);
        let on = hot(pinned, &b);
        if on {
            p.round(b.left, b.top, b.right, b.bottom, p.px(6.0), TRACK);
        }
        let glyph = if pinned { "\u{E77A}" } else { "\u{E718}" };
        p.centred(&b, glyph, if on { TEXT } else { MUTED }, ICON);
    }
    b.bottom + p.px(8.0)
}

// A line above, then "Open dashboard" and the external-link glyph, and on the right how
// old the figures are; the whole line is the link (buttons::set_footer). Returns the
// widget's bottom.
unsafe fn footer(p: &Painter, m: &Model, pinned: bool, w: i32, y: i32, draw: bool) -> i32 {
    let y = y + p.px(4.0);
    let r = RECT { left: 0, top: y + 1, right: w, bottom: y + 1 + p.px(32.0) };
    if draw {
        p.fill(p.px(1.0).max(1), y, w - p.px(1.0).max(1), y + 1, BORDER);
        let (pad, cy) = (p.px(14.0), (r.top + r.bottom) / 2);
        let c = if hot(pinned, &r) { TEXT } else { MUTED };
        p.text_mid(pad, cy, "Open dashboard", c, NORMAL);
        p.text_mid(pad + p.width_of("Open dashboard", NORMAL) + p.px(6.0), cy, "\u{E8A7}", c, ICON);
        if let Some(t) = m.checked {
            let a = ago(t, m.now);
            p.text_mid(w - pad - p.width_of(&a, SMALL), cy, &a, MUTED, SMALL);
        }
        set_footer(pinned, r);
    }
    r.bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    // Everything on: both sides on one account, a usable and a full other account.
    fn full_model() -> Model {
        let u = Use { five: Win { u: 16, reset: Some(9_000_000) }, week: Win { u: 40, reset: Some(200_000_000) } };
        let mut m = Model { now: 1_000_000, checked: Some(820_000), ..Default::default() };
        m.cli = Side { email: Some("a@x.com".into()), usage: Some(u.clone()) };
        m.desk = Some(m.cli.clone());
        m.others.push(Other { email: "b@x.com".into(), desk_uuid: Some("u".into()), usage: Some(u), ..Default::default() });
        m.others.push(Other { email: "c@x.com".into(), ..Default::default() });
        m
    }

    #[test]
    fn layout_puts_the_dc_font_back() {
        unsafe {
            let dc = CreateCompatibleDC(ptr::null_mut());
            let before = GetCurrentObject(dc, OBJ_FONT as u32);
            assert!(layout(&full_model(), 1.0, false, dc) > 0);
            assert_eq!(GetCurrentObject(dc, OBJ_FONT as u32), before);
            DeleteDC(dc);
        }
    }

    #[test]
    fn layout_is_as_tall_drawn_as_measured() {
        unsafe {
            let dc = CreateCompatibleDC(ptr::null_mut());
            for s in [1.0, 1.25, 1.5, 2.0] {
                for (m, pinned) in [(full_model(), false), (Model::default(), true)] {
                    assert_eq!(layout(&m, s, pinned, ptr::null_mut()), layout(&m, s, pinned, dc));
                }
            }
            DeleteDC(dc);
        }
    }

    #[test]
    fn ago_is_short() {
        assert_eq!(ago(1_000, 1_000), "updated just now");
        assert_eq!(ago(5_000, 1_000), "updated just now");
        assert_eq!(ago(0, 3 * 60_000 + 59_000), "updated 3 min ago");
        assert_eq!(ago(0, 2 * 3_600_000 + 60_000), "updated 2 h ago");
        assert_eq!(ago(0, 3 * 86_400_000), "updated 3 d ago");
    }

    // The header names the app (the figures' age is in the footer).
    #[test]
    fn header_names_the_app() {
        assert_eq!(TITLE, words::APP);
    }
}
