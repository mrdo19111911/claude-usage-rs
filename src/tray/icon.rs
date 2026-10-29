// The tray icon: added, kept up to date, drawn as two bars.

use super::*;

// ---------- The icon ----------

pub(super) fn nid() -> NOTIFYICONDATAW {
    let mut n: NOTIFYICONDATAW = unsafe { mem::zeroed() };
    n.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    n.hWnd = ui(|u| u.msg);
    n.uID = 1;
    n
}

pub(super) fn tip(n: &mut NOTIFYICONDATAW, m: &Model) {
    let t = match (&m.cli.email, &m.cli.usage) {
        (Some(e), Some(u)) => format!("Claude CLI: {e}\n5-hour {}% · weekly {}%", u.five.u, u.week.u),
        (Some(e), None) => format!("Claude CLI: {e}"),
        _ => "Claude usage".into(),
    };
    for (d, s) in n.szTip.iter_mut().zip(t.encode_utf16().take(127).chain(Some(0))) {
        *d = s;
    }
}

pub(super) unsafe fn add_icon() {
    update_icon(true);
}

// The icon draws the Claude CLI account's two limits as two bars, in the page's colors.
pub(super) unsafe fn update_icon(add: bool) {
    let m = ui(|u| u.model.clone());
    let dpi = GetDpiForSystem();
    let size = GetSystemMetricsForDpi(SM_CXSMICON, dpi).max(16);
    let (five, week) = m.cli.usage.as_ref().map_or((None, None), |u| (Some(u.five.u), Some(u.week.u)));
    let key = format!("{size}/{five:?}/{week:?}");
    if !add && ui(|u| u.icon_key == key) {
        return;
    }
    let icon = make_icon(size, five, week);
    // No icon made: keep the old one and try again next time.
    if icon.is_null() && !add {
        return;
    }
    let mut n = nid();
    n.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
    n.uCallbackMessage = WM_TRAY;
    n.hIcon = icon;
    tip(&mut n, &m);
    if add {
        // After TaskbarCreated the icon may still be there: then update it.
        if Shell_NotifyIconW(NIM_ADD, &n) == 0 {
            Shell_NotifyIconW(NIM_MODIFY, &n);
        }
        n.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_SETVERSION, &n);
    } else {
        Shell_NotifyIconW(NIM_MODIFY, &n);
    }
    let old = ui(|u| {
        u.icon_key = key;
        mem::replace(&mut u.icon, icon)
    });
    if !old.is_null() {
        DestroyIcon(old);
    }
}

// Opaque ARGB from a COLORREF (0x00bbggrr).
fn argb(c: u32) -> u32 {
    0xff00_0000 | (c & 0xff) << 16 | (c & 0xff00) | (c >> 16 & 0xff)
}

// Bytes of a 1-bpp mask: each row padded to a 16-bit word.
fn mask_len(size: i32) -> usize {
    ((size as usize + 15) / 16 * 2) * size as usize
}

// Top bar: 5-hour limit; bottom bar: weekly. Gray while there are no figures.
fn icon_pixels(size: i32, five: Option<i64>, week: Option<i64>) -> Vec<u32> {
    let n = size as usize;
    let mut px = vec![0u32; n * n];
    let r = (n / 4) as f32;
    for y in 0..n {
        for x in 0..n {
            // Rounded square: outside a corner's quarter circle stays transparent.
            let (cx, cy) = ((x as f32 + 0.5).clamp(r, n as f32 - r), (y as f32 + 0.5).clamp(r, n as f32 - r));
            if (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) <= r * r {
                px[y * n + x] = argb(rgb(0x26, 0x26, 0x28));
            }
        }
    }
    let m = (n / 6).max(2);
    let gap = (n / 8).max(1);
    let bh = (n - 2 * m - gap) / 2;
    let inner = n - 2 * m;
    for (i, u) in [five, week].into_iter().enumerate() {
        let y0 = m + i * (bh + gap);
        let fill = u.map_or(0, |u| (inner as i64 * u.clamp(0, 100) as i64 + 99) / 100) as usize;
        let fill = if u.is_some_and(|u| u > 0) { fill.max(1) } else { fill };
        for y in y0..y0 + bh {
            for x in m..m + inner {
                let c = if x - m < fill { level(u.unwrap_or(0)) } else if u.is_some() { rgb(0x4a, 0x4a, 0x4e) } else { rgb(0x5a, 0x5a, 0x5e) };
                px[y * n + x] = argb(c);
            }
        }
    }
    px
}

pub(super) unsafe fn make_icon(size: i32, five: Option<i64>, week: Option<i64>) -> HICON {
    let px = icon_pixels(size, five, week);
    let mut bmi: BITMAPINFO = mem::zeroed();
    bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut core::ffi::c_void = ptr::null_mut();
    let color = CreateDIBSection(ptr::null_mut(), &bmi, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
    if color.is_null() || bits.is_null() {
        if !color.is_null() {
            DeleteObject(color);
        }
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(px.as_ptr(), bits as *mut u32, px.len());
    // All-zero mask: the alpha channel alone decides what shows.
    let zeros = vec![0u8; mask_len(size)];
    let mask = CreateBitmap(size, size, 1, 1, zeros.as_ptr() as *const _);
    let info = ICONINFO { fIcon: TRUE, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
    let icon = CreateIconIndirect(&info);
    DeleteObject(color);
    DeleteObject(mask);
    icon
}
