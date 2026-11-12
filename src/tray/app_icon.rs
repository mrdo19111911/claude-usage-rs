// The exe's own icon, assets/claude-usage.ico (docs/specs/exe-resources.md): the tray
// icon's drawing at the sizes Windows asks for, written as an .ico. Test-only: build.rs
// embeds the file; `cargo test write_app_icon -- --ignored` redraws it.

use super::*;

pub(super) const SIZES: [i32; 6] = [16, 24, 32, 48, 64, 256];
// What the app icon shows: 5-hour 60% (green), weekly 80% (amber).
const FIVE: i64 = 60;
const WEEK: i64 = 80;

fn u16le(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn u32le(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

// One image: BITMAPINFOHEADER (height doubled: colour + mask), bottom-up BGRA rows,
// then an all-zero 1-bpp AND mask (alpha alone decides), rows padded to 32 bits.
fn dib(size: i32, px: &[u32]) -> Vec<u8> {
    let n = size as usize;
    let mut v = Vec::new();
    u32le(&mut v, 40);
    u32le(&mut v, size as u32);
    u32le(&mut v, 2 * size as u32);
    u16le(&mut v, 1);
    u16le(&mut v, 32);
    for _ in 0..6 {
        u32le(&mut v, 0);
    }
    for y in (0..n).rev() {
        for &p in &px[y * n..(y + 1) * n] {
            // ARGB as u32 little-endian is B, G, R, A: what a DIB wants.
            u32le(&mut v, p);
        }
    }
    v.resize(v.len() + (n + 31) / 32 * 4 * n, 0);
    v
}

// The whole .ico: ICONDIR, one 16-byte entry per size, then the images.
pub(super) fn ico_bytes() -> Vec<u8> {
    let images: Vec<Vec<u8>> = SIZES.iter().map(|&s| dib(s, &icon_pixels(s, Some(FIVE), Some(WEEK)))).collect();
    let mut v = Vec::new();
    u16le(&mut v, 0);
    u16le(&mut v, 1);
    u16le(&mut v, SIZES.len() as u16);
    let mut at = 6 + 16 * SIZES.len();
    for (s, img) in SIZES.iter().zip(&images) {
        // 256 is written as 0.
        let b = (*s % 256) as u8;
        v.extend_from_slice(&[b, b, 0, 0]);
        u16le(&mut v, 1);
        u16le(&mut v, 32);
        u32le(&mut v, img.len() as u32);
        u32le(&mut v, at as u32);
        at += img.len();
    }
    images.iter().for_each(|i| v.extend_from_slice(i));
    v
}

pub(super) fn asset_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("claude-usage.ico")
}
