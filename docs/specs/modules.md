# Module layout (spec)

Rule: every file under `src/` and `tests/` has **at most 200 lines** (`tests/file_size.rs`
checks it). A file near the limit moves its tests to a sibling file:
`#[cfg(test)] #[path = "x_tests.rs"] mod tests;` (or `tests.rs` inside a module folder).

Splitting does not change behaviour: every existing test keeps passing, unchanged in
what it checks. Code is moved, not rewritten. Items become `pub(crate)` only where
another module needs them.

## Who owns what (one subagent per row; no file is shared)

| Area | Files after the split | Interface other areas rely on (must not change) |
|---|---|---|
| core (was main.rs) | `main.rs` (startup only), `app.rs`, `model.rs`, `creds.rs`, `anthropic.rs`, `poll.rs`, `switch.rs`, `signin.rs`, `names.rs`, `state_json.rs`, `server.rs`, `util.rs` (names may vary, each ≤200) | `crate::open_browser(&str)`; `App::public_state(&self) -> String` |
| tray (was tray.rs) | `tray/mod.rs`, `tray/model.rs`, `tray/icon.rs`, `tray/place.rs`, `tray/widget.rs`, `tray/paint.rs` | `tray::start(StateFn, url, data_dir)`, `tray::request_exit()`, `tray::signal_running() -> bool` |
| desktop (was desktop.rs, lock.rs) | `desktop/mod.rs`, `desktop/store.rs`, `desktop/switch.rs`, `desktop/win.rs`, `lock.rs` + `lock_tests.rs` | everything `pub` in desktop.rs and lock.rs today, same paths (`desktop::Store`, `desktop::signed_in`, `lock::refresh`, ...) |
| web (was index.html) | `web.rs`, `web/index.html`, `web/style.css`, `web/*.js` | `web::asset(path) -> Option<(&'static str, &'static str)>` |

## Tray behaviour (what the split must keep)

- No console window ever; started from a terminal it prints there, then detaches.
- One instance (named mutex). A second launch asks the running one to open the dashboard.
- Listens on a random free port (PORT overrides); no browser tab at start (`--open` does).
- Tray icon: two bars (Claude CLI account's 5-hour and weekly use), page colours.
- Hover the icon: widget above it — Claude CLI, Claude Desktop, other accounts; reset
  times as time left plus this computer's local clock time ("4h39m · 22:20").
  Pointer away from icon and widget for 3 ticks of 150 ms: widget hides.
- Click icon or widget: dashboard. Right-click icon: Open dashboard / Pin widget / Exit.
- Pinned widget: always on top, drags anywhere, ✕ unpins; place kept in `widget.json`.
- Exit (menu or dashboard button): icon removed from the tray, then the program ends.
- Exit while a Claude Desktop switch runs waits for it (killed midway it skips its
  rollback): the menu says so in a box; the 2 s refresh exits once the switch is done.

## Switching from the widget

- Each row under "Other accounts" has two small buttons on its right: **CLI** and
  **Desktop**. CLI is missing when that account is Claude CLI's now; Desktop is missing
  when it is Claude Desktop's now, when it has no saved Desktop sign-in (first time goes
  through the dashboard), when Claude Desktop is not on this computer, and while a Desktop
  switch runs.
- CLI switches at once (`Act::Cli(id)`, same as the dashboard's "Use here").
- Desktop asks first (Yes/No box: the app closes and opens again, running tasks stop),
  then `Act::Desktop(uuid)`.
- The switch runs off the tray thread (it calls Anthropic); the widget redraws when it
  ends; a failure shows its message in a box. A click outside the buttons still opens the
  dashboard (hover widget) as before.
- Tests first: the model fills `Other` from the state JSON; the button rectangles and the
  hit test are pure functions.

## Test-first parts

Pure functions get their tests written before they are moved or changed: state JSON →
widget model, local clock text, time-left text, icon pixels, widget placement, the
hover-hide decision, `web::asset` for every file.
