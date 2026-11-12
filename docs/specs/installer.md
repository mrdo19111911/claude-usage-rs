# Installer (spec)

A `setup.exe` made with Inno Setup 6 (`installer/scas.iss`, built by
`installer/build.ps1` into `target/installer/SCAS-Setup-<version>.exe`, the version
taken from Cargo.toml). Why these choices: `installer/SO_SANH_INNO.md`.

- Install mode (test: `tests/installer_iss.rs`), chosen on Setup's first page:
  - **All users** (default, asks for admin): `C:\Program Files\SCAS\scas.exe`,
    shortcut in the common Start menu (`C:\ProgramData\Microsoft\Windows\Start Menu\Programs`).
  - **Only for me** (no admin): `%LOCALAPPDATA%\Programs\SCAS\scas.exe`,
    shortcut in the user's Start menu (`%APPDATA%\...\Start Menu\Programs`).
  - Why all users is the default: on some Windows 10 22H2 machines Start stops listing new
    shortcuts in the user's Start menu folder while it still lists new common ones
    (measured 2026-11-11/12: per-user probes never reach `shell:AppsFolder`, an all-users
    probe does). The cause on Windows' side is still open.
  - An upgrade keeps the installed copy's mode (`UsePreviousPrivileges`, Inno's default);
    a self-update of an all-users copy therefore shows a UAC prompt. `/ALLUSERS` or
    `/CURRENTUSER` on the command line picks a mode without the page.
  - `{autopf}`, `{autoprograms}` and `Root: HKA` follow the mode; no fixed per-user or
    per-machine path is left in the script.
- Start menu shortcut "Smartlog Claude Auto Switch". Task "Start with Windows" (on by default): Run value
  `SCAS` (HKLM for all users, HKCU for one user), removed on uninstall.
- Last page: "Open Smartlog Claude Auto Switch now" starts it with `--open` (dashboard tab once).
- The app is always started as the user who ran Setup (`runasoriginaluser`), never as the
  elevated admin account: it reads that user's Claude files.
- Upgrade over a running copy: setup unpacks the new exe to a temp folder and runs
  `scas.exe --quit`; if the mutex `Local\claude-usage-dashboard` is still held,
  setup stops with a message instead of overwriting a locked exe.
- Uninstall: `--quit` with the installed exe first; still running → message, no uninstall.
  User data (`%USERPROFILE%\.claude-usage-dashboard`, saved sign-ins) is kept.
- Listed in Apps & features as "Smartlog Claude Auto Switch" with its uninstaller.
- Renamed from "Claude Usage" (`docs/specs/rename.md`): Setup uses `UsePreviousAppDir=no` and
  removes the 1.5.x leftovers: old folder `{autopf}\Claude Usage`, old shortcut
  `{autoprograms}\Claude Usage.lnk`, old Run value `ClaudeUsage`.

## `--quit` (tray/instance.rs)

- Finds the tray window of its kind (`cud-tray`, or `cud-tray-demo` with `--demo`), posts
  the same exit message as the dashboard's Exit button (a Claude Desktop switch under way
  finishes first, the tray icon is removed), and waits up to 60 s for that process to end.
- Exit code 0 when none runs (also when none ran), 1 when it still runs.
- Test first: with no such window it returns true at once.
