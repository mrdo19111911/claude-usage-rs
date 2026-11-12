// Smooth lines over a GDI DC through GDI+'s flat API (docs/SO_SANH_GADGET_CHART.md,
// method C, after lastwarhq-scanner's src/ui/paint.rs): anti-aliased, f32 points, half-
// pixel offset. Text stays in GDI (ClearType). Colours come in as GDI COLORREFs.

use super::*;
use std::sync::OnceLock;
use windows_sys::Win32::Graphics::GdiPlus::*;

// GDI+ is started once per process, on first use, and left running (the process ends it).
fn started() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| unsafe {
        let mut token = 0usize;
        let input = GdiplusStartupInput { GdiplusVersion: 1, ..Default::default() };
        GdiplusStartup(&mut token, &input, ptr::null_mut()) == Ok
    })
}

// A COLORREF (0x00BBGGRR) as GDI+'s ARGB, with alpha `a`.
pub(super) fn argb(c: u32, a: u8) -> u32 {
    (a as u32) << 24 | (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff)
}

// A GDI+ drawing surface on a DC; deleted when dropped.
pub(super) struct Gfx(*mut GpGraphics);

impl Gfx {
    pub(super) unsafe fn on(dc: HDC) -> Option<Gfx> {
        if dc.is_null() || !started() {
            return None;
        }
        let mut g = ptr::null_mut();
        if GdipCreateFromHDC(dc, &mut g) != Ok {
            return None;
        }
        GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
        GdipSetPixelOffsetMode(g, PixelOffsetModeHalf);
        Some(Gfx(g))
    }

    unsafe fn pen(&self, c: u32, width: f32, dash: bool, f: impl FnOnce(*mut GpPen)) {
        let mut pen = ptr::null_mut();
        if GdipCreatePen1(c, width, UnitPixel, &mut pen) != Ok {
            return;
        }
        GdipSetPenLineJoin(pen, LineJoinRound);
        if dash {
            GdipSetPenDashStyle(pen, DashStyleDash);
        }
        f(pen);
        GdipDeletePen(pen);
    }

    // A polyline through `pts`, colour `c` (ARGB).
    pub(super) unsafe fn lines(&self, pts: &[(f32, f32)], c: u32, width: f32) {
        if pts.len() < 2 {
            return;
        }
        let v: Vec<PointF> = pts.iter().map(|&(x, y)| PointF { X: x, Y: y }).collect();
        self.pen(c, width, false, |pen| {
            GdipDrawLines(self.0, pen, v.as_ptr(), v.len() as i32);
        });
    }

    pub(super) unsafe fn line(&self, a: (f32, f32), b: (f32, f32), c: u32, width: f32, dash: bool) {
        self.pen(c, width, dash, |pen| {
            GdipDrawLine(self.0, pen, a.0, a.1, b.0, b.1);
        });
    }

    // A filled dot of radius `r` at (x, y).
    pub(super) unsafe fn dot(&self, x: f32, y: f32, r: f32, c: u32) {
        let mut br = ptr::null_mut();
        if GdipCreateSolidFill(c, &mut br) == Ok {
            GdipFillEllipse(self.0, br as *mut GpBrush, x - r, y - r, 2.0 * r, 2.0 * r);
            GdipDeleteBrush(br as *mut GpBrush);
        }
    }
}

impl Drop for Gfx {
    fn drop(&mut self) {
        unsafe { GdipDeleteGraphics(self.0) };
    }
}
