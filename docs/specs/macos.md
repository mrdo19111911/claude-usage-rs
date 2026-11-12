# macOS build (spec)

Why this way: `docs/SO_SANH_MACOS_BUILD.md`. Until now only Windows was built and tested.

## What runs on macOS

- The dashboard in the browser (`http://127.0.0.1:4777`, `PORT` sets it; the browser opens
  at start, `--no-open` stops that) and the terminal it was started from ("Keep this
  window open. Ctrl+C to stop.").
- Claude Code (CLI, VS Code) accounts: usage figures, adding accounts, switching by hand
  and auto-switch. Claude Code keeps its login in the macOS Keychain
  (`Claude Code-credentials`, read and written with `security`; `CUD_NO_KEYCHAIN` falls
  back to `~/.claude/.credentials.json`). Same code as before (`creds.rs`).

## What stays Windows-only, and how macOS says so

- Tray icon and widget (`src/tray/`): not built; no `--quit` (Ctrl+C ends it).
- Claude Desktop sign-in switching, its token and its 4:00 renewal (`src/desktop/`,
  `desktop_renew.rs`): `desktop::SWITCHES = cfg!(windows)`. `/api/state` gains
  `desktop.supported` (= `SWITCHES`). `find_data_dir` finds nothing on macOS (it only
  looks under `%LOCALAPPDATA%` / `%APPDATA%`), so `desktop.found` is false; the Claude
  Desktop panel then reads "Switching Claude Desktop works on Windows only." instead of
  "Claude Desktop was not found on this computer."
- Installing updates (`src/update/`): `update::INSTALLS = cfg!(windows)`; the release's
  setup is `SCAS-Setup-X.Y.Z.exe`, a Windows installer.
  - The check still runs (every 3 days, on demand) and still shows "Version X available"
    with the release page link.
  - `begin_install` refuses when `INSTALLS` is false, before anything is downloaded, with
    "Installing updates works on Windows only; download the new version from the release
    page." The status stays "available".
  - Automatic install never starts; a found version then starts the 3 days like
    `autoUpdate` off (`waits_to_install(newer, auto, installs)` = all three true).
  - `/api/state` gains `canInstallUpdates` (= `INSTALLS`). The page: with it false, no
    "Install X" button and no "Install updates automatically" box; the "Update to X" pill
    opens the release page instead of installing.
- Gemini's `cgemini.cmd` launcher is a Windows batch file (follow-up, not in this change).

## Build

- `.github/workflows/macos.yml` on `macos-latest`: `cargo test` on the runner's own arch,
  then release builds for `aarch64-apple-darwin` and `x86_64-apple-darwin`, joined with
  `lipo -create` into one universal `scas`, packed as
  `scas-<version>-macos-universal.tar.gz` (tar keeps the exec bit; the artifact
  upload would zip it away) and uploaded as a workflow artifact.
- From Windows (no Mac): `python tools/build_macos.py` (why: `macos-build-prior-art.md`).
  - Needs `python -m pip install --user ziglang` and `cargo install cargo-zigbuild`;
    finds zig in the `ziglang` package, so it need not be on PATH.
  - Runs `cargo zigbuild --release --locked --target universal2-apple-darwin` with
    `MACOSX_DEPLOYMENT_TARGET=11.0` into `target/mac-zig`.
  - Checks the result (`tools/macho.py`) and stops with the problems if any: a fat Mach-O
    with exactly the arm64 and x86_64 slices, each an executable, the arm64 one carrying
    a code signature (Apple Silicon kills unsigned code), minos 11.0, and every dylib
    under `/usr/lib/`.
  - Packs `dist/scas-<version>-macos-universal.tar.gz` holding one file `scas`, mode 0755,
    and prints its path, size and SHA-256. Linked and inspected only; the CI run is what
    executes it.
- Not signed or notarized: after downloading, `xattr -d com.apple.quarantine scas`
  (or right-click → Open once) before the first run.
- Checked on Windows without a Mac: `cargo check --target aarch64-apple-darwin
  --all-targets` with a stand-in C compiler (the C parts are not linked by a check).

## SCAS.app and the .dmg

Why this way: `macos-build-prior-art.md` (second table).

- `SCAS.app/Contents/`:
  - `Info.plist`: `CFBundleName` SCAS, `CFBundleDisplayName` Smartlog Claude Auto
    Switch, `CFBundleExecutable` scas-launcher, `CFBundleIdentifier` io.github.mrdo19111911.scas,
    both version keys = Cargo.toml's version, `CFBundlePackageType` APPL,
    `CFBundleIconFile` scas, `LSMinimumSystemVersion` 11.0, `LSUIElement` true (no Dock
    icon or menu bar: SCAS has no window; the browser is its window),
    `NSHighResolutionCapable` true.
  - `MacOS/scas-launcher`: the launcher, a `/bin/sh` script (LF line ends). Not named
    `SCAS`: macOS volumes are case-insensitive by default, so it would overwrite `scas`. Port = `PORT` or 4777.
    - If `GET http://127.0.0.1:<port>/api/state` answers within 2 s: `open` that address
      (a second double-click reopens the dashboard), log "open", exit 0.
    - Otherwise: start `MacOS/scas` with `nohup`, in its own process group (`set -m`),
      stdin from /dev/null, output appended to `~/Library/Logs/SCAS/scas.log`; log
      "start", exit 0. `scas` then opens the browser itself, as from a terminal.
    - Its own lines go to `~/Library/Logs/SCAS/launcher.log` (date, word, port).
  - `MacOS/scas`: the universal binary, unchanged.
  - `Resources/scas.icns`: copied from `assets/scas.icns`, which `tools/make_icns.py`
    makes from `assets/logo.jpg` with `make_icon.py`'s drawing (PNG entries 16 to 1024
    px; no `iconutil`, so it runs on Windows).
- Quitting: the dashboard's Exit button (`POST /api/exit`), as before.
- `tools/build_macos_app.py [--binary PATH] [--out DIR]` (default binary: the zig build's
  `target/mac-zig/universal2-apple-darwin/release/scas`, default out `dist/`):
  - checks the binary with `macho.problems` first and stops on any;
  - writes `DIR/SCAS.app` (on macOS with real modes) and
    `DIR/SCAS-<version>-macos-app.tar.gz`, whose entries carry the modes themselves:
    directories and the two files in `MacOS/` 0755, the rest 0644 (Windows has no chmod).
- `.github/workflows/macos.yml` (push to `macos-ci`, or by hand): after the universal
  binary, runs this script, signs the bundle ad hoc (`codesign --force --deep --sign -`,
  then `--verify --deep --strict`), and makes `SCAS-<version>.dmg` with
  `hdiutil create -volname SCAS -fs HFS+ -srcfolder <stage> -format UDZO` where the stage
  holds `SCAS.app` and an `Applications` link to `/Applications`. Smoke test on the
  runner: mount the dmg, check both entries, copy the app out, `open` it, wait for
  `/api/state`; the server still answers after the launcher is gone; `open` again → the
  launcher log's last word is "open" and there is still exactly one `scas` process; then
  `POST /api/exit` (with `X-CUD: 1`) and no `scas` process is left. Uploads the dmg, the
  binary tar.gz and the app tar.gz.
- The dmg is unsigned and not notarized: after copying, `xattr -dr
  com.apple.quarantine /Applications/SCAS.app`, or System Settings → Privacy & Security
  → Open Anyway after the first try.

## Tests

- `tools/test_build_macos_app.py`, `tools/test_icns.py` (Windows and the runner).
- `update/platform_tests.rs`: refusal before download, status kept; `waits_to_install`.
- `state_json`: `canInstallUpdates` and `desktop.supported` equal `cfg!(windows)`, so the
  same test asserts the Windows answer on Windows and the macOS one on the Mac runner.
