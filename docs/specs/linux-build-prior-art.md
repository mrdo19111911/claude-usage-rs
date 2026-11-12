# Linux build: prior art (2026-11-12)

Goal: a Linux `scas` (x86_64, static musl preferred) built from this Windows machine,
and the same in CI. C parts to compile: bundled SQLite (`rusqlite`), `ring` (rustls).

This machine: no WSL distro (`wsl -l` lists none), no Docker, no Linux `gcc`.

| Route | Method | Fit here | Cost |
|---|---|---|---|
| **[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)** (chosen) | `zig cc` as C compiler and linker; ships musl and any glibc version (`x86_64-unknown-linux-gnu.2.17`) | Works on Windows today: `pip install ziglang cargo-zigbuild`, no admin, no VM. Built the static musl binary here | ~50 MB zig from PyPI; one more tool in CI |
| WSL `cargo build` | Native Linux toolchain in a WSL distro | Can also *run* the binary. No distro installed; installing one needs admin and a reboot | Distro + rustup + gcc inside it |
| [cross](https://github.com/cross-rs/cross) | Docker images with a full cross toolchain per target | No Docker here. Good when tests must run under emulation | Docker Desktop, image pulls (~1 GB) |
| GitHub Actions `ubuntu-latest` | Native Linux runner; `cargo test` runs for real | Best for tests and the smoke run. Needs a push (origin is public; not done) | Free minutes; push policy |
| rustup `rust-lld` only | `linker = "rust-lld"` for musl | Fails here: cc-rs still needs a C compiler for SQLite and ring | none, but not enough |

Choice: cargo-zigbuild for the binary, both locally (`tools/build_linux.py`) and in CI
(`.github/workflows/linux.yml`), with the CI runner also running `cargo test` and a
smoke test (`--demo --no-open`, curl `/api/state`).
