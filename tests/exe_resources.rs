// The built exe carries its icon and version info (docs/specs/exe-resources.md).
#![cfg(windows)]

use std::ptr;
use windows_sys::Win32::UI::Shell::ExtractIconExW;

const EXE: &str = env!("CARGO_BIN_EXE_scas");

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn holds(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn exe_has_an_icon() {
    let path: Vec<u16> = EXE.encode_utf16().chain(Some(0)).collect();
    // Index -1 with no output arrays: the number of icons in the file.
    let n = unsafe { ExtractIconExW(path.as_ptr(), -1, ptr::null_mut(), ptr::null_mut(), 0) };
    assert!(n >= 1, "{EXE} has {n} icons");
}

#[test]
fn exe_has_version_info() {
    let bytes = std::fs::read(EXE).unwrap();
    for s in ["VS_VERSION_INFO", "ProductName", "Smartlog Claude Auto Switch", "scas.exe", env!("CARGO_PKG_VERSION")] {
        assert!(holds(&bytes, &utf16(s)), "no UTF-16 {s:?} in {EXE}");
    }
}
