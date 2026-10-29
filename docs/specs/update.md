# Version, help, check for updates, automatic update (spec)

Why this way: `docs/SO_SANH_UPDATE.md`. Version 1.1.0 is the first with these.

## Version

- One source: Cargo.toml `version`; in code `update::VERSION = env!("CARGO_PKG_VERSION")`.
- Shown: the dashboard's top bar ("Claude Usage 1.1.0"), the tray menu's first line
  (greyed "Claude Usage 1.1.0"), Apps & features (the setup's AppVersion, build.ps1).

## Help

- `help.html` (served like index.html, same style.css): short answers, one `<details>`
  each: what the two limits are; the bars, % left, the pace tick, "not started yet";
  model limits (Fable); Claude CLI vs Claude Desktop; switching and auto-switch; adding
  accounts (CLI and Desktop); how often figures are checked, HTTP 429; the widget (hover,
  pin, buttons); updates; where data is kept (tokens never leave this computer except to
  Anthropic); uninstall.
- Links: the dashboard's top bar "Help", the tray menu "Help" (opens `<url>/help.html`).

## Check for updates

- `GET https://api.github.com/repos/mrdo19111911/claude-usage-rs/releases/latest`
  (User-Agent "claude-usage/<version>", Accept application/vnd.github+json), 20 s timeout.
- The release: `tag_name` "vX.Y.Z" → X.Y.Z; newer when its numbers are greater than
  VERSION's, compared part by part (`newer`). Drafts and pre-releases are not "latest".
- Its setup: the asset named `ClaudeUsage-Setup-X.Y.Z.exe`; `digest` must be
  "sha256:<64 hex>" (no digest: not installable, said so). `pick_asset` returns name, url,
  size, sha256.
- When: 1 minute after start, then every 6 hours; and on demand (dashboard button, tray
  menu). Not in the demo.
- Status shown in /api/state:
  `"version": "1.1.0", "update": {"status": S, "latest": "1.2.0"|null, "url": release
  page|null, "checkedAt": ms|null, "error": text|null}`, S one of "idle" (never checked),
  "checking", "upToDate", "available", "downloading", "installing", "error".
  Settings gain `autoUpdate` (default true; old saved settings without it: true).

## Install (by hand or automatic)

- Download the setup to `%TEMP%\claude-usage-update\` (follows GitHub's redirect), check
  size and SHA-256 against the release's digest; a mismatch deletes it and reports
  "error". Only github.com / objects.githubusercontent.com URLs from the API answer.
- Run it detached from this process: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART
  /RELAUNCH=1`. The setup closes this app the usual way (`--quit`: waits for a Claude
  Desktop switch), installs, then starts the new version (`[Run]` entry with
  `skipifnotsilent` and `Check: Relaunch`, `{param:RELAUNCH|0}` = "1").
- Automatic: when a check finds a newer version and `autoUpdate` is on, it installs at
  once unless a Claude Desktop switch or a sign-in is under way (then the next check).
- Events log: "Update 1.2.0 found", "Installing update 1.2.0", errors.

## Where it shows

- Dashboard top bar: "Claude Usage 1.1.0 · Help"; when an update is available a pill
  "Update to 1.2.0" (click = install, after a confirm).
- Settings card, "Updates" row: version, status text ("Up to date · checked 5 min ago",
  "Version 1.2.0 available · What's new", "Downloading…", error), buttons "Check for
  updates" / "Install 1.2.0", checkbox "Install updates automatically".
- Tray menu: greyed "Claude Usage 1.1.0", "Open dashboard", "Help", "Check for updates",
  separator, "Pin widget…", separator, "Exit". "Check for updates" answers in a box: up to
  date, or "Version X is available. Install now?" (Yes installs), or the error.

## Interfaces between parts

- Rust core (`src/update/`): `VERSION`; `newer(a, b) -> bool`; `pick_asset(&Value) ->
  Result<Asset, String>`; `sha256_hex(&[u8]) -> String`; `App::check_update(&self) ->
  Result<(), String>` (sets the status); `App::install_update(&self) -> Result<(), String>`
  (downloads, verifies, starts the setup; this app is then closed by the setup);
  routes `POST /api/update/check`, `POST /api/update/install`; `autoUpdate` in
  `POST /api/settings`.
- Tray: `Act::CheckUpdate`, `Act::InstallUpdate` (switch.rs tray_hooks calls the two
  App methods); after CheckUpdate the tray reads `update` from the state JSON.
- Tests first: `newer` (1.10.0 > 1.9.0, equal, "v" prefix, bad text = not newer);
  `pick_asset` (right asset by name, missing digest = error, bad digest = error);
  `sha256_hex` known value; settings without `autoUpdate` read as true; web: help.html
  served and linked, update.js served, the top bar shows the version; the iss has the
  relaunch entry.
