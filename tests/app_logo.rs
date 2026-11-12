// The logo as the app's icon (docs/specs/exe-resources.md): the .ico the exe embeds, the
// dashboard's tab icon, and Setup's icon. Made by tools/make_icon.py from assets/logo.jpg.

use std::fs;
use std::path::PathBuf;

const SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

#[test]
fn the_ico_lists_every_size_at_32_bits() {
    let b = fs::read(root().join("assets/scas.ico")).unwrap();
    assert_eq!((u16_at(&b, 0), u16_at(&b, 2)), (0, 1), "an icon file");
    let n = u16_at(&b, 4) as usize;
    let mut sizes = vec![];
    for k in 0..n {
        let e = 6 + 16 * k;
        sizes.push(if b[e] == 0 { 256 } else { b[e] as u32 });
        assert_eq!(u16_at(&b, e + 6), 32, "32 bpp");
        let (len, at) = (u32_at(&b, e + 8) as usize, u32_at(&b, e + 12) as usize);
        assert!(at + len <= b.len(), "image {k} inside the file");
        let img = &b[at..at + len];
        assert!(img.starts_with(b"\x89PNG") || u32_at(img, 0) == 40, "image {k}: PNG or DIB");
    }
    sizes.sort();
    assert_eq!(sizes, SIZES);
}

#[test]
fn the_logo_it_is_made_from_is_kept() {
    let jpg = fs::read(root().join("assets/logo.jpg")).unwrap();
    assert!(jpg.starts_with(&[0xFF, 0xD8]), "a JPEG");
}

#[test]
fn the_pages_use_it_as_their_tab_icon() {
    let png = fs::read(root().join("src/web/img/app-icon.png")).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    for page in ["src/web/index.html", "src/web/help.html"] {
        let t = fs::read_to_string(root().join(page)).unwrap();
        assert!(t.contains(r#"<link rel="icon" type="image/png" href="img/app-icon.png">"#), "{page}");
    }
    let web = fs::read_to_string(root().join("src/web.rs")).unwrap();
    assert!(web.contains(r#""/img/app-icon.png" => Some(include_bytes!("web/img/app-icon.png"))"#), "served");
}

#[test]
fn setup_shows_it_too() {
    let iss = fs::read_to_string(root().join("installer/scas.iss")).unwrap();
    assert!(iss.lines().any(|l| l.trim() == r"SetupIconFile=..\assets\scas.ico"));
}
