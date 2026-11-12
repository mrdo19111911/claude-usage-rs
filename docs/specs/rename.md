# Rename to Smartlog Claude Auto Switch, SCAS (spec)

Asked 2026-11-12: first "đổi tên thành claude autoswitch" (every visible place, Windows and
macOS), then, the same hour and before any release, "Smartlog Claude Auto Switch = SCAS":
the full name where people read it, SCAS for files. The logo is smartlog's
(docs/specs/exe-resources.md). The GitHub repo keeps its name (`mrdo19111911/claude-usage-rs`).

## New names

- Shown name "Smartlog Claude Auto Switch": dashboard title and top bar, Help title, tray
  tooltip, tray version line and update messages, dashboard update prompt, "Dashboard
  stopped" note, cgemini messages, the exe's VERSIONINFO (`ProductName`, `FileDescription`),
  Setup (`AppName`, Start menu shortcut, Apps & features, last page, its messages).
- Program: Cargo package `scas`, so `scas.exe` on Windows and `scas` on macOS;
  `OriginalFilename` "scas.exe".
- Setup: `installer/scas.iss` builds `SCAS-Setup-<version>.exe`; installed in
  `{autopf}\SCAS`; Run value `SCAS`.
- macOS release file: `scas-<version>-macos-universal.tar.gz`.
- Exported usage log: `scas-log-<yyyymmdd>.jsonl`, `"source": "scas"`.
- Update download folder `%TEMP%\scas-update`; GitHub User-Agent `scas/<version>`; icon file
  `assets/scas.ico`.
- "Claude AutoSwitch" (`claude-autoswitch.exe`, `ClaudeAutoSwitch-Setup-…`) was never
  released: nothing has to be removed or upgraded from it.

## Kept (so nothing is lost and old copies upgrade in place)

- Setup `AppId` {63905BC9-FAEF-420D-8FC1-DF04B915B4AF}: the same entry in Apps & features.
- Data folder `%USERPROFILE%\.claude-usage-dashboard` (and `CUD_DATA_DIR`): saved accounts,
  sign-ins and settings stay where they are.
- Single-instance mutex `Local\claude-usage-dashboard` and tray window class `cud-tray`: the
  new exe's `--quit` still ends a running old copy, and old and new never run together.
- The repo URL, so the updater's `releases/latest` address is unchanged.

## Upgrade from 1.5.x

- 1.5.x updaters download the asset named `ClaudeUsage-Setup-<version>.exe`, so each release
  also carries the new setup under that name (same bytes) for as long as 1.5.x copies exist.
- Setup uses `UsePreviousAppDir=no`: it installs into the new folder, then removes the old
  folder `{autopf}\Claude Usage` (old exe and old uninstaller), the old shortcut
  `{autoprograms}\Claude Usage.lnk`, and the old Run value `ClaudeUsage` (HKA). The old copy
  is asked to quit first (`--quit` of the new exe, by window class), as before.

## Tests (written before the change)

- `tests/app_name.rs`: neither 1.5.x's name nor "Claude AutoSwitch" is left in the app's
  code, web pages, build script, installer, build.ps1 and macOS workflow, except the kept
  names above and the old names Setup removes.
- `tests/installer_rename.rs`, `tests/installer_iss.rs`: new names, folder `SCAS`, kept AppId
  and mutex, `UsePreviousAppDir=no`, the three removals of 1.5.x leftovers.
- `tests/exe_resources.rs`: VERSIONINFO holds "Smartlog Claude Auto Switch" and "scas.exe".
- Unit tests: update asset name, tray texts, page titles, cgemini text, export file name.
