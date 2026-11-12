// Gives the Windows exe its icon and version info (docs/specs/exe-resources.md): writes a
// .rc into OUT_DIR, as Zed does (docs/SO_SANH_EXE_RESOURCES.md), and compiles it in.

use std::{env, fs, path::PathBuf};

fn rc_text(icon: &str, version: &str, nums: [&str; 3]) -> String {
    let [a, b, c] = nums;
    let icon = icon.replace('\\', "\\\\");
    // Numbers, not winver.h names: no SDK include path needed.
    // 0x40004 = VOS_NT_WINDOWS32, 0x1 = VFT_APP.
    format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {a},{b},{c},0
PRODUCTVERSION {a},{b},{c},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "mrdo19111911"
      VALUE "FileDescription", "Smartlog Claude Auto Switch"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "scas"
      VALUE "OriginalFilename", "scas.exe"
      VALUE "ProductName", "Smartlog Claude Auto Switch"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    )
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/scas.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = dir.join("assets").join("scas.ico");
    let v = |k: &str| env::var(k).unwrap();
    let text = rc_text(
        &icon.to_string_lossy(),
        &v("CARGO_PKG_VERSION"),
        [&v("CARGO_PKG_VERSION_MAJOR"), &v("CARGO_PKG_VERSION_MINOR"), &v("CARGO_PKG_VERSION_PATCH")],
    );
    let rc = PathBuf::from(v("OUT_DIR")).join("scas.rc");
    fs::write(&rc, text).unwrap();
    embed_resource::compile(&rc, embed_resource::NONE).manifest_optional().unwrap();
}
