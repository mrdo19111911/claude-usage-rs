# Spec: the exe carries its icon and version

## Why
`scas.exe` was built bare: 0 icons (`ExtractIconEx`), empty `ProductName` and
`FileDescription`. Start, the taskbar, Apps & features and Task Manager had no icon or name of its
own to show. How others do it: [../SO_SANH_EXE_RESOURCES.md](../SO_SANH_EXE_RESOURCES.md).

## What
- The software's logo (given 2026-11-12: https://vinasa.org.vn/Anhdaidien/smartlog1.jpg, the
  "smartlog" wordmark, blue on white, 350x225) is kept as `assets/logo.jpg`.
- `tools/make_icon.py` (tests: `tools/test_make_icon.py`) makes from it, on a white rounded
  square with a clear (alpha 0) outside:
  - 16, 24, 32, 48, 64 px: the wordmark's first letter "s", alone (the word can't be read there;
    taskbar, Start and the tray menu use these);
  - 256 px: the whole logo, wordmark and tagline (Explorer's large icons, Apps & features);
  - written as `assets/scas.ico` (those 6 sizes, 32-bit), and the 64 px image as
    `src/web/img/app-icon.png`, the dashboard's and Help's tab icon (`<link rel="icon">`).
  Rebuild after changing the logo: `python tools/make_icon.py`.
- Setup shows the same icon (`SetupIconFile`). The tray icon keeps its live bars (5-hour and
  weekly use): it is a gauge, not the logo.
- `build.rs`: when the target is Windows, writes a `.rc` into `OUT_DIR` (`1 ICON` = that file,
  `1 VERSIONINFO` from `CARGO_PKG_VERSION`) and compiles it in with `embed-resource` 3.
- VERSIONINFO: `ProductName` and `FileDescription` "Smartlog Claude Auto Switch", `CompanyName` "mrdo19111911",
  `OriginalFilename` "scas.exe", `FileVersion` / `ProductVersion` = the Cargo version.

## Input / output
- Input: `Cargo.toml` version, `assets/scas.ico`.
- Output: every built `scas.exe` (debug and release) has >= 1 icon and that version info.

## Tests (written first)
- `tests/exe_resources.rs` (Windows): the exe Cargo built for the tests
  (`CARGO_BIN_EXE_scas`) has >= 1 icon (`ExtractIconExW`) and its bytes hold, in UTF-16,
  `VS_VERSION_INFO`, `Smartlog Claude Auto Switch` and the Cargo version.
- `tools/test_make_icon.py`: sizes, clear corners, logo blue in the middle, the small sizes are
  the "s" alone (taller than wide), the large ones the word (much wider than tall).
- `tests/app_logo.rs`: `assets/scas.ico` lists the 6 sizes at 32 bpp, each image
  inside the file; the page and Help link `img/app-icon.png`, which is served; Setup's
  `SetupIconFile` is that `.ico`.

## Done when
Tests green, every file at most 200 lines; the release exe measured: icon count >= 1, ProductName
"Smartlog Claude Auto Switch", FileVersion = Cargo version; reinstalled, then checked whether Start lists it.
