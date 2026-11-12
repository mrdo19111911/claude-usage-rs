# Building the macOS binary: prior art and choice (2026-11-12)

Goal: a universal (arm64 + x86_64) `scas` for macOS, made from this Windows machine
(no Mac, no WSL distro, no Docker, no clang) or from CI. SCAS has C code (bundled
SQLite, ring) built by `cc`, and links no Apple framework (ureq uses rustls with
bundled roots on macOS), so only `libSystem` / `libiconv` are needed at link time.

| Route (prior art) | How | Fit for SCAS here | Cost |
|---|---|---|---|
| GitHub Actions `macos-latest` + `lipo` (ripgrep, Alacritty, taiki-e/upload-rust-binary-action; details in `docs/SO_SANH_MACOS_BUILD.md`) | arm64 runner builds both targets, `lipo -create`, `codesign -s -` | Best check: real Mac runs `cargo test` and a smoke test of both slices. Already in `.github/workflows/macos.yml`. Blocked today: origin is public and holds only `main` (publish.py leaves out `.github/`, tests, specs) | Free runner minutes; needs a push |
| [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) (`universal2-apple-darwin` target) | `zig cc` compiles the C parts and links against Zig's own libSystem stubs; cargo-zigbuild joins the two slices | Works on Windows: zig from PyPI (`ziglang` 0.17.1), `cargo install cargo-zigbuild` (0.24.1). Measured: 1m40s cold, 6.4 MB fat binary, arm64 slice ad-hoc signed by the linker, minos 11.0. No SDK needed because no framework is linked (README: frameworks need `SDKROOT`). Cannot run the result | Two small installs; nothing else |
| [osxcross](https://github.com/tpoechtrager/osxcross) | Clang + Apple's `ld64`/cctools ported to Linux, with an SDK packed from Xcode | README lists Linux and BSD hosts only; needs a WSL distro (none here) and an Xcode SDK tarball made on a Mac | High: toolchain build + SDK |
| Docker images with osxcross/zig (e.g. `ghcr.io/rust-cross/cargo-zigbuild`, `joseluisq/rust-linux-darwin-builder`) | Same tools prebuilt, SDK included | Needs Docker (not installed); adds nothing over native cargo-zigbuild for SCAS | Docker install, multi-GB images |

## Choice

- **Make the binary here with cargo-zigbuild** (`tools/build_macos.py`): one command,
  checks the Mach-O it made (both slices, arm64 signed, minos 11.0, only `/usr/lib`
  dylibs) and packs `dist/scas-<version>-macos-universal.tar.gz` with the exec bit set.
- **Keep the CI workflow** as the run test on a real Mac; it is the only route that
  executes the binary. Until it runs, the zig-built binary is linked and inspected,
  not run.

## Measured here (2026-11-12)

- `cargo check --target {aarch64,x86_64}-apple-darwin --all-targets` (stand-in C
  compiler): 0 errors, 0 warnings on `a8637e0` (WebView2 dashboard commit); `wry`,
  `src/tray/` and `tray::request_open` are already behind `cfg(windows)`.
- `cargo zigbuild --release --target universal2-apple-darwin` with
  `MACOSX_DEPLOYMENT_TARGET=11.0`: builds; rustc warns that `xcrun` is missing (no SDK
  path), harmless here. Slices: x86_64 3 265 336 B, arm64 3 107 552 B; both link
  `/usr/lib/libSystem.B.dylib`, `libiconv.2.dylib`, `libcharset.1.dylib`.
- arm64 signature checked by hand: CodeDirectory flags `0x20002` (ad-hoc, linker-signed),
  all 753 SHA-256 page hashes match the slice's bytes, so `lipo` joining did not break it.

## Sources

- https://github.com/rust-cross/cargo-zigbuild (README: universal2 target, SDKROOT, frameworks)
- https://github.com/tpoechtrager/osxcross (README: hosts, SDK packaging)
- `docs/SO_SANH_MACOS_BUILD.md` (CI workflows of ripgrep, bat, cargo-dist, Alacritty, taiki-e)

# SCAS.app in a .dmg: prior art and choice (2026-11-12)

Goal: ship `SCAS.app` inside `SCAS-<version>.dmg` (drag to Applications), next to the
tar.gz. SCAS is a local web server with no Cocoa event loop: run directly as a bundle's
executable it would show a Dock icon that never answers ("not responding"), and a second
double-click would only re-activate it, so the dashboard could not be reopened.

| Prior art | How the .app / .dmg is made | Fit for SCAS | Cost |
|---|---|---|---|
| [Alacritty](https://github.com/alacritty/alacritty) `Makefile` (`app`, `dmg` targets) | Copies a template `Alacritty.app` (Info.plist, icns), puts the binary in `Contents/MacOS`, `codesign --remove-signature` then `codesign --force --deep --sign -`; `ln -sf /Applications`, `hdiutil create -volname … -fs HFS+ -srcfolder … -ov -format UDZO` | **Taken for the .dmg**: plain `hdiutil`, works headless on a runner, no extra tool | None beyond Xcode CLT on the runner |
| [create-dmg](https://github.com/create-dmg/create-dmg) | Shell script around `hdiutil` plus an AppleScript that lays out the Finder window (background, icon positions, `--app-drop-link`) | Prettier window, but the AppleScript needs a GUI session; on CI it is skipped with `--skip-jenkins`, which leaves Alacritty's plain result | Homebrew install; flaky AppleScript on CI |
| [cargo-bundle](https://github.com/burtonageo/cargo-bundle) | `[package.metadata.bundle]` in Cargo.toml (identifier, icon, `osx.minimum_system_version`, `info_plist_exts`); converts PNG icons; signs .app and .dmg | Makes the bundle's executable the Rust binary itself, so the Dock problem stays; README says "very early alpha"; adds metadata to Cargo.toml (shared file) | New tool and Cargo.toml keys |
| Script-as-executable bundles: [Mathias Bynens, "shell script apps"](https://mathiasbynens.be/notes/shell-script-mac-apps), [Platypus](https://sveinbjorn.org/platypus_documentation) ("runs in background" = `LSUIElement`) | `Contents/MacOS/<Name>` is a shell script; Mathias's example starts a local web server and opens the browser; Platypus registers background apps as `LSUIElement` (no Dock icon, no menu bar) | **Taken for the launcher**: a script that opens the dashboard if SCAS answers, else starts `scas` detached and exits; `LSUIElement` true | A few lines of sh |

Choice:
- Bundle assembled by our own `tools/build_macos_app.py` (runs on Windows and on the
  runner; tar entries carry the exec bits, since Windows has no chmod), icon from a
  committed `assets/scas.icns` made by `tools/make_icns.py` (PNG entries, no `iconutil`).
- `.dmg` made on the macOS runner the Alacritty way (`hdiutil`, UDZO, `/Applications`
  link). No create-dmg.
- Not found in the sources: what a second double-click does to a background server. The
  launcher answers it by asking the port each time; the CI smoke test measures it.
