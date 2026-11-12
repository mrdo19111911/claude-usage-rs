# Linux build (spec)

Why this way: `docs/specs/linux-build-prior-art.md`. Same non-Windows mode as macOS
(`docs/specs/macos.md`); no Linux-only code was needed.

## What runs on Linux

- The dashboard in the browser at `http://127.0.0.1:4777` (`PORT` sets it), opened with
  `xdg-open` at start (`--no-open` stops that; without a desktop `xdg-open` just fails
  quietly and the URL is printed). Keep the terminal open; Ctrl+C stops it.
- Claude Code (CLI, VS Code) accounts: usage figures, adding accounts, switching by hand
  and auto-switch. Claude Code on Linux keeps its login in `~/.claude/.credentials.json`
  (`$CLAUDE_CONFIG_DIR/.credentials.json` when set): `keychain` is false on every OS
  but macOS, so `creds.rs` reads and writes that file. Files SCAS writes get mode 0600
  (`util.rs`, `audit.rs`, `usage_log.rs`, `#[cfg(unix)]`).
- TLS: rustls with the bundled roots plus the OS store (`native-certs`, `/etc/ssl`).

## What does not

- No tray icon or widget, no dashboard window, no `--quit` (`src/tray/`, Windows only).
- No Claude Desktop switching: there is no official Claude Desktop for Linux, and
  `find_data_dir` only looks under `%LOCALAPPDATA%`/`%APPDATA%`. `desktop.supported` is
  false; the panel says switching works on Windows only.
- No installing updates (`update::INSTALLS` false): the check runs and links the
  release page.
- Gemini's `cgemini.cmd` launcher is a Windows batch file.

## Build

- Local, any host: `python tools/build_linux.py` (needs `rustup target add
  x86_64-unknown-linux-musl` and `pip install ziglang cargo-zigbuild`). Output:
  `target/linux/x86_64-unknown-linux-musl/release/scas`, one static x86_64 ELF (no
  `PT_INTERP`), checked by the script. `python tools/build_linux.py
  x86_64-unknown-linux-gnu.2.17` gives a glibc >= 2.17 one instead.
- CI: `.github/workflows/linux.yml` on `ubuntu-latest`: `cargo test`, the script's own
  tests and build, a smoke run (`--demo --no-open`, `/api/state` must say
  `canInstallUpdates` false and `desktop.supported` false), and
  `scas-<version>-linux-x86_64-musl.tar.gz` as a workflow artifact.

## Tests

- `tools/test_build_linux.py`: ELF check (static vs dynamic, arch, not-ELF), the cargo
  command, output path, zig put on PATH once.
- The Rust tests are the same as on macOS; `state_json` asserts `cfg!(windows)` for
  `canInstallUpdates` and `desktop.supported`, so the Linux runner checks the Linux answer.
