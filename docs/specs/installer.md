# Installer (spec)

A `setup.exe` made with Inno Setup 6 (`installer/claude-usage.iss`, built by
`installer/build.ps1` into `target/installer/ClaudeUsage-Setup-<version>.exe`, the version
taken from Cargo.toml). Why these choices: `installer/SO_SANH_INNO.md`.

- Per user, never asks for admin: `%LOCALAPPDATA%\Programs\Claude Usage\claude-usage.exe`.
- Start menu shortcut "Claude Usage". Task "Start with Windows" (on by default): HKCU Run
  value `ClaudeUsage`, removed on uninstall.
- Last page: "Open Claude Usage now" starts it with `--open` (dashboard tab once).
- Upgrade over a running copy: setup unpacks the new exe to a temp folder and runs
  `claude-usage.exe --quit`; if the mutex `Local\claude-usage-dashboard` is still held,
  setup stops with a message instead of overwriting a locked exe.
- Uninstall: `--quit` with the installed exe first; still running → message, no uninstall.
  User data (`%USERPROFILE%\.claude-usage-dashboard`, saved sign-ins) is kept.
- Listed in Apps & features as "Claude Usage" with its uninstaller.

## `--quit` (tray/instance.rs)

- Finds the tray window of its kind (`cud-tray`, or `cud-tray-demo` with `--demo`), posts
  the same exit message as the dashboard's Exit button (a Claude Desktop switch under way
  finishes first, the tray icon is removed), and waits up to 60 s for that process to end.
- Exit code 0 when none runs (also when none ran), 1 when it still runs.
- Test first: with no such window it returns true at once.
