// Drawing primitives for the widget (GDI): text, fills, rounded shapes. What goes where:
// layout.rs (header, footer, order) and sections.rs (cards and rows).

use super::*;

pub(super) struct Painter {
    pub(super) dc: HDC,
    pub(super) s: f32,
    pub(super) fonts: [HFONT; 4],
}

pub(super) const NORMAL: usize = 0;
pub(super) const BOLD: usize = 1;
pub(super) const SMALL: usize = 2;
// Segoe MDL2 Assets: pin, unpin, external link.
pub(super) const ICON: usize = 3;

impl Painter {
    pub(super) fn px(&self, v: f32) -> i32 {
        (v * self.s).round() as i32
    }
    pub(super) unsafe fn fill(&self, l: i32, t: i32, r: i32, b: i32, c: u32) {
        let br = CreateSolidBrush(c);
        FillRect(self.dc, &RECT { left: l, top: t, right: r, bottom: b }, br);
        DeleteObject(br);
    }
    // A filled rounded rectangle, corner radius `r` (pixels).
    pub(super) unsafe fn round(&self, l: i32, t: i32, r: i32, b: i32, rad: i32, c: u32) {
        let br = CreateSolidBrush(c);
        let old_b = SelectObject(self.dc, br);
        let old_p = SelectObject(self.dc, GetStockObject(NULL_PEN));
        RoundRect(self.dc, l, t, r + 1, b + 1, 2 * rad, 2 * rad);
        SelectObject(self.dc, old_p);
        SelectObject(self.dc, old_b);
        DeleteObject(br);
    }
    // A rounded outline, 1 px, nothing filled.
    pub(super) unsafe fn outline(&self, l: i32, t: i32, r: i32, b: i32, rad: i32, c: u32) {
        let pen = CreatePen(PS_SOLID, 1, c);
        let old_p = SelectObject(self.dc, pen);
        let old_b = SelectObject(self.dc, GetStockObject(NULL_BRUSH));
        RoundRect(self.dc, l, t, r, b, 2 * rad, 2 * rad);
        SelectObject(self.dc, old_b);
        SelectObject(self.dc, old_p);
        DeleteObject(pen);
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
