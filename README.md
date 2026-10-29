# Claude Usage

A Windows tray app that shows Claude usage (5-hour and weekly limits) for several
accounts and switches the Claude CLI and Claude Desktop sign-in between them.

## Install

Download `ClaudeUsage-Setup-<version>.exe` from
[Releases](https://github.com/mrdo19111911/claude-usage-rs/releases) and run it. No admin
rights needed. Uninstall from Settings → Apps; saved accounts in
`%USERPROFILE%\.claude-usage-dashboard` are kept.

## Build

Rust (stable, MSVC) and, for the setup, [Inno Setup 6](https://jrsoftware.org/isinfo.php):

```
cargo build --release
powershell -ExecutionPolicy Bypass -File installer\build.ps1
```

The setup lands in `target\installer\`.
