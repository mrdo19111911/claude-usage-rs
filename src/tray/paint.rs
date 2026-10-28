// Drawing the widget with GDI.

use super::*;

pub(super) struct Painter {
    pub(super) dc: HDC,
    pub(super) s: f32,
    pub(super) fonts: [HFONT; 3],
}

pub(super) const NORMAL: usize = 0;
pub(super) const BOLD: usize = 1;
pub(super) const SMALL: usize = 2;

impl Painter {
    pub(super) fn px(&self, v: f32) -> i32 {
        (v * self.s).round() as i32
    }
    pub(super) unsafe fn fill(&self, l: i32, t: i32, r: i32, b: i32, c: u32) {
        let br = CreateSolidBrush(c);
        FillRect(self.dc, &RECT { left: l, top: t, right: r, bottom: b }, br);
        DeleteObject(br);
    }
    pub(super) unsafe fn width_of(&self, s: &str, font: usize) -> i32 {
        SelectObject(self.dc, self.fonts[font]);
        let t: Vec<u16> = s.encode_utf16().collect();
        let mut sz = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(self.dc, t.as_ptr(), t.len() as i32, &mut sz);
        sz.cx
    }
    // Cut with "…" to fit `max` pixels.
    pub(super) unsafe fn fitted(&self, s: &str, font: usize, max: i32) -> String {
        if self.width_of(s, font) <= max {
            return s.into();
        }
        let mut c: Vec<char> = s.chars().collect();
        while !c.is_empty() && self.width_of(&format!("{}…", c.iter().collect::<String>()), font) > max {
            c.pop();
        }
        format!("{}…", c.iter().collect::<String>())
    }
    pub(super) unsafe fn text(&self, x: i32, y: i32, s: &str, c: u32, font: usize) {
        SelectObject(self.dc, self.fonts[font]);
        SetTextColor(self.dc, c);
        let t: Vec<u16> = s.encode_utf16().collect();
        TextOutW(self.dc, x, y, t.as_ptr(), t.len() as i32);
    }
    pub(super) unsafe fn text_right(&self, r: i32, y: i32, s: &str, c: u32, font: usize) {
        self.text(r - self.width_of(s, font), y, s, c, font);
    }
}

// Draws the widget when `dc` is set; returns its height either way.
pub(super) unsafe fn layout(m: &Model, s: f32, pinned: bool, dc: HDC) -> i32 {
    let w = width(s);
    let font = |h: f32, weight: i32| {
        if dc.is_null() {
            return ptr::null_mut();
        }
        CreateFontW(-(h * s).round() as i32, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, wide("Segoe UI").as_ptr())
    };
    let p = Painter { dc, s, fonts: [font(13.0, 400), font(13.0, 600), font(11.5, 400)] };
    let draw = !dc.is_null();
    let pad = p.px(14.0);
    let mut y = p.px(10.0);
    // The DC's own font, put back before ours are deleted.
    let old = if draw { SelectObject(dc, p.fonts[NORMAL]) } else { ptr::null_mut() };
    if draw {
        SetBkMode(dc, TRANSPARENT as i32);
        p.fill(0, 0, w, 10_000, BG);
        p.text(pad, y, "CLAUDE USAGE", MUTED, SMALL);
        let b = button(s, w);
        p.fill(b.left, b.top, b.right, b.bottom, rgb(0x2a, 0x2a, 0x2d));
        let label = if pinned { "✕" } else { "Pin" };
        p.text(b.left + (b.right - b.left - p.width_of(label, SMALL)) / 2, b.top + p.px(4.0), label, TEXT, SMALL);
    }
    y += p.px(28.0);

    let side = |y: &mut i32, title: &str, note: &str, color: u32, sd: &Side| {
        if draw {
            p.text(pad, *y, title, color, BOLD);
            p.text_right(w - pad, *y + p.px(1.5), note, MUTED, SMALL);
            let email = sd.email.as_deref().map_or("Not signed in".to_string(), |e| p.fitted(e, NORMAL, w - 2 * pad));
            p.text(pad, *y + p.px(19.0), &email, if sd.email.is_some() { TEXT } else { MUTED }, NORMAL);
        }
        *y += p.px(42.0);
        if let Some(u) = &sd.usage {
            for (label, win) in [("5h", &u.five), ("Week", &u.week)] {
                if draw {
                    let (bl, br) = (pad + p.px(42.0), w - pad - p.px(158.0));
                    let (bt, bb) = (*y + p.px(6.0), *y + p.px(13.0));
                    p.text(pad, *y, label, MUTED, SMALL);
                    p.fill(bl, bt, br, bb, TRACK);
                    let f = ((br - bl) as i64 * win.u.clamp(0, 100) / 100) as i32;
                    p.fill(bl, bt, bl + if win.u > 0 { f.max(p.px(3.0)) } else { 0 }, bb, level(win.u));
                    p.text_right(w - pad - p.px(114.0), *y - p.px(1.0), &format!("{}%", win.u), TEXT, BOLD);
                    let reset = win.reset.filter(|&t| t > m.now).map_or(String::new(), |t| format!("{} · {}", left(t - m.now), clock(t, m.now)));
                    p.text_right(w - pad, *y, &reset, MUTED, SMALL);
                }
                *y += p.px(20.0);
            }
        }
        *y += p.px(8.0);
    };
    side(&mut y, "Claude CLI", "terminal · VS Code", CODE, &m.cli);
    if let Some(d) = &m.desk {
        if draw {
            p.fill(pad, y - p.px(4.0), w - pad, y - p.px(3.0), BORDER);
        }
        y += p.px(4.0);
        side(&mut y, "Claude Desktop", "own sign-in", DESK, d);
    }

    let mut hits = vec![];
    if !m.others.is_empty() {
        if draw {
            p.fill(pad, y - p.px(4.0), w - pad, y - p.px(3.0), BORDER);
            p.text(pad, y + p.px(2.0), "OTHER ACCOUNTS", MUTED, SMALL);
        }
        y += p.px(24.0);
        for o in &m.others {
            let (email, u) = (&o.email, &o.usage);
            // The switch buttons at the right; the text ends left of them.
            let bs = row_buttons(m, o, w, s, y);
            let right = text_right(w, s);
            if draw {
                let (figs, c) = match u {
                    Some(u) if u.week.u >= 100 => (format!("week full until {}", u.week.reset.map_or("?".into(), |t| clock(t, m.now))), RED),
                    Some(u) if u.five.u >= 100 => (format!("5h full until {}", u.five.reset.map_or("?".into(), |t| clock(t, m.now))), RED),
                    Some(u) => (format!("5h {}% · week {}%", u.five.u, u.week.u), if u.week.u >= 90 || u.five.u >= m.threshold { AMBER } else { GREEN }),
                    None => ("no figures yet".into(), MUTED),
                };
                let fw = p.width_of(&figs, SMALL);
                let name = email.split('@').next().unwrap_or(email);
                p.text(pad, y, &p.fitted(name, NORMAL, right - pad - fw - p.px(10.0)), TEXT, NORMAL);
                p.text_right(right, y + p.px(1.5), &figs, c, SMALL);
                draw_buttons(&p, &bs);
                hits.extend(bs);
            }
            y += p.px(22.0);
        }
        y += p.px(6.0);
    }
    if draw {
        // Where the widget finds the buttons on a click.
        set_hits(pinned, hits);
        let hint = if pinned { "Drag to move · double-click: dashboard" } else { "Click: dashboard · right-click the icon: menu" };
        p.text(pad, y, hint, MUTED, SMALL);
    }
    y += p.px(26.0);
    if draw {
        // Border last, over everything.
        let br = CreateSolidBrush(BORDER);
        FrameRect(dc, &RECT { left: 0, top: 0, right: w, bottom: y }, br);
        DeleteObject(br);
        SelectObject(dc, old);
        for f in p.fonts {
            DeleteObject(f);
        }
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_puts_the_dc_font_back() {
        unsafe {
            let dc = CreateCompatibleDC(ptr::null_mut());
            let before = GetCurrentObject(dc, OBJ_FONT as u32);
            let mut m = Model::default();
            m.cli.email = Some("a@x.com".into());
            m.cli.usage = Some(Use::default());
            m.desk = Some(Side::default());
            m.others.push(Other { email: "b@x.com".into(), desk_uuid: Some("u".into()), ..Default::default() });
            assert!(layout(&m, 1.0, false, dc) > 0);
            assert_eq!(GetCurrentObject(dc, OBJ_FONT as u32), before);
            DeleteDC(dc);
        }
    }
}
