# Spec: the exe carries its icon and version

## Why
`claude-usage.exe` was built bare: 0 icons (`ExtractIconEx`), empty `ProductName` and
`FileDescription`. Start, the taskbar, Apps & features and Task Manager had no icon or name of its
own to show. How others do it: [../SO_SANH_EXE_RESOURCES.md](../SO_SANH_EXE_RESOURCES.md).

## What
- `assets/claude-usage.ico`: the tray icon's drawing (`icon_pixels`, two bars on a dark rounded
  square; 5-hour 60%, weekly 80%) at 16, 24, 32, 48, 64 and 256 px, 32-bit with alpha.
- `build.rs`: when the target is Windows, writes a `.rc` into `OUT_DIR` (`1 ICON` = that file,
  `1 VERSIONINFO` from `CARGO_PKG_VERSION`) and compiles it in with `embed-resource` 3.
- VERSIONINFO: `ProductName` and `FileDescription` "Claude Usage", `CompanyName` "mrdo19111911",
  `OriginalFilename` "claude-usage.exe", `FileVersion` / `ProductVersion` = the Cargo version.

## Input / output
- Input: `Cargo.toml` version, `assets/claude-usage.ico`.
- Output: every built `claude-usage.exe` (debug and release) has >= 1 icon and that version info.

## Tests (written first)
- `tests/exe_resources.rs` (Windows): the exe Cargo built for the tests
  (`CARGO_BIN_EXE_claude-usage`) has >= 1 icon (`ExtractIconExW`) and its bytes hold, in UTF-16,
  `VS_VERSION_INFO`, `Claude Usage` and the Cargo version.
- `src/tray/app_icon.rs`: the `.ico` header lists the 6 sizes; the file in `assets/` is byte for byte
  what `icon_pixels` draws now (regenerate: `cargo test write_app_icon -- --ignored`).

## Done when
Tests green, every file at most 200 lines; the release exe measured: icon count >= 1, ProductName
"Claude Usage", FileVersion = Cargo version; reinstalled, then checked whether Start lists it.
