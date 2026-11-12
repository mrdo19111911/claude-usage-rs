use super::*;

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

#[test]
fn header_lists_every_size_and_images_fit() {
    let b = ico_bytes();
    assert_eq!((u16_at(&b, 0), u16_at(&b, 2), u16_at(&b, 4)), (0, 1, SIZES.len() as u16));
    let mut end = 6 + 16 * SIZES.len();
    for (k, &s) in SIZES.iter().enumerate() {
        let e = 6 + 16 * k;
        let w = if b[e] == 0 { 256 } else { b[e] as i32 };
        assert_eq!(w, s);
        assert_eq!(u16_at(&b, e + 6), 32);
        let (len, at) = (u32_at(&b, e + 8) as usize, u32_at(&b, e + 12) as usize);
        assert_eq!(at, end, "images follow one another");
        // BITMAPINFOHEADER: width, doubled height, 32 bpp.
        assert_eq!((u32_at(&b, at), u32_at(&b, at + 4), u32_at(&b, at + 8)), (40, s as u32, 2 * s as u32));
        let n = s as usize;
        assert_eq!(len, 40 + 4 * n * n + (n + 31) / 32 * 4 * n);
        end = at + len;
    }
    assert_eq!(end, b.len());
}

#[test]
fn rows_are_stored_bottom_up() {
    let b = ico_bytes();
    let at = u32_at(&b, 6 + 12) as usize + 40;
    let px = icon_pixels(16, Some(FIVE), Some(WEEK));
    // The first stored row is the drawing's last; its first pixel is a clear corner.
    assert_eq!(u32_at(&b, at), px[15 * 16]);
    let m = (16 / 6).max(2);
    // The top bar's first pixel sits in stored row 15 - m.
    assert_eq!(u32_at(&b, at + 4 * ((15 - m) * 16 + m)), px[m * 16 + m]);
    assert_ne!(px[m * 16 + m], 0);
}

#[test]
fn asset_matches_the_tray_drawing() {
    let on_disk = fs::read(asset_path()).expect("assets/claude-usage.ico missing: cargo test write_app_icon -- --ignored");
    assert!(on_disk == ico_bytes(), "assets/claude-usage.ico is stale: cargo test write_app_icon -- --ignored");
}

#[test]
#[ignore]
fn write_app_icon() {
    let p = asset_path();
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, ico_bytes()).unwrap();
}
