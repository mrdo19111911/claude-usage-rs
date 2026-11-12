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
- Listens on the port of its last run again (kept in `port` next to state.json), a random
  free one only when that is taken (PORT overrides); so a `cgemini` session open across a
  restart (an update, a sign-out) carries on: seen 2026-10-29, ECONNREFUSED in Claude Code
  after a restart moved the port. The demo (`--demo`, which runs beside the real one with
  the same data folder) neither reads nor writes that file: always a random free port, so
  it never takes the real one's port and `cgemini` never reaches the demo. No browser tab
  at start (`--open` does).
- Tray icon: two bars (Claude CLI account's 5-hour and weekly use), page colours.
- Hover the icon: widget above it — Claude CLI, Claude Desktop, other accounts; reset
  times as time left plus this computer's local clock time ("resets in 4h39m · 22:20").
  Pointer away from icon and widget for 3 ticks of 150 ms: widget hides.
- Click icon or widget: dashboard, in its own WebView2 window (tray/dash.rs, dash_spot.rs,
  dash_rect.rs, dash_nav.rs; specs/dashboard-window.md), the browser without WebView2.
  Right-click icon: greyed version line / Open dashboard /
  Help / Check for updates / Keep widget on screen / Exit (stops auto-switch) (tray/menu.rs;
  updates: specs/update.md). Every widget and tray string: tray/words.rs.
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

## The "Claude accounts" section: two lists, like "In use now"

- Two columns: **Claude CLI** on the left (`#cliRows`), **Claude Desktop** on the right
  (`#deskRows`); one column under the other on narrow screens. No shared table.
- Claude CLI list: one row per CLI login (`S.accounts`): auto-switch toggle, email, plan,
  status, last/next check, 5-hour and weekly bars, **Use here** or **✓ In use**, ✕ remove
  (not on the one in use). Then accounts known only to Claude Desktop, muted, with
  **Sign in** (adds a CLI login). The column head holds "+ Add account" (`#cliAdd`).
- Claude Desktop list: one row per saved Desktop account (`S.desktop.accounts`): email,
  5-hour and weekly bars (its CLI account's figures when there is one), **Use here**
  (saved sign-in) or **✓ In use**, ✕ forget (not on the one signed in). Then CLI accounts
  with no Desktop sign-in yet, muted, with **Use here (first time)**. The wait box
  (`#desktopSub`) sits at the top of this column. Desktop not found, or the demo: a note
  instead of rows. Buttons are disabled while a Desktop switch runs.
- Buttons keep their data attributes (`data-activate`, `data-signin`, `data-remove`,
  `data-toggle`, `data-dswitch`, `data-dfirst`, `data-dremove`); main.js listens on the
  section (`#accts`), so `run()` is unchanged.
- Files: `web/js/rows.js` (shared: accountRows, checkLine, usageBars, renderRows = summary
  + both lists), `web/js/list_cli.js` (renderCli), `web/js/list_desk.js` (renderDesk).
- Tests first (web_tests.rs): both lists in the page, CLI before Desktop, both scripts
  served, no six-column table left.

## Widget v2 (tray/): smaller, no repeats, most useful first

What the user saw as wrong in v1: CLI and Desktop on one account drew the same email and
bars twice; 360 px wide; the only usable account sat 4th among long red "week full"
lines; flat coloured buttons with no hover, ragged columns, buttons even on full
accounts; square corners; "✕" unclear; a hint line always on.

- 300 px wide (logical), rounded window corners (region, radius 10), 1 px border.
- Header: the app's name "Smartlog Claude Auto Switch" (`words::APP`), pin icon at
  the right (Segoe MDL2 Assets: pin U+E718 on the hover widget, unpin U+E77A pinned).
- "IN USE NOW" title, then the "In use" card (darker fill, radius 8): one card when Claude CLI and Claude Desktop are on
  the same account (`same_account`), with both chips (CLI teal, Desktop violet, outlined);
  otherwise one card per side with its chip. Email without the domain when it fits.
  Each window: label ("5-hour"/"Weekly"), bold %, time left · clock at the right, then a
  6 px rounded bar under it with a pace tick (`pace`: share of the window gone; light
  line); "no use yet" when a limit has no reset time. No usage: "Usage not checked yet". Desktop missing: no Desktop card or chip.
- "SWITCH TO": usable accounts (`split_others().0`: not full), most room first; name, two
  small stacked bars (5-hour over weekly), "26% · 74%" under a "5-hour · weekly" key, and
  outlined Code / Desktop buttons in
  fixed columns (a missing one leaves its slot empty). Hovered button: filled.
- "USED UP · AVAILABLE AT" (was "FULL · FREE AGAIN", reworded 2026-11-12): full accounts (`split_others().1`), soonest free first; muted
  name and the time it can be used again ("not known" when unknown); no buttons.
- Footer: "Open dashboard" + external-link glyph (U+E8A7), and on the right "updated N min ago" (newest figures, `Model.checked`, small, muted); click opens the dashboard.
  The hover widget still opens it on any click outside buttons; the pinned one drags.
- Tests first: pace, full_until, split_others order, same_account, `checked`; button
  slots and hit test; the layout leaves the DC font in place; a preview test
  (`cargo test preview -- --ignored`) renders presets to target/widget-preview/*.bmp.

## Add buttons and a new account's chart

What the user saw: "+ Add account" too small to notice; no way to add an account to Claude
Desktop that Claude CLI does not know; a newly added account's chart showed two tiny dots
at its right edge (Anthropic gives only the figures now, history starts when it is added).

- Both column heads hold "+ Add account" (`#cliAdd`, `#deskAdd`), larger than a row button
  and outlined in the column's colour (teal / violet), filled on hover.
- `#deskAdd` runs the first-time Desktop sign-in with no email (`/api/desktop/add`): Claude
  Desktop opens signed out, any account signed in there is saved. Disabled in the demo,
  when Claude Desktop is not found, and while a Desktop switch runs.
- Chart: the figures now are drawn as two larger dots (radius 5.5) at the right edge with
  their % labels, also when the account has no line yet (no "Not enough figures" text when
  there are figures). When the account's first figures fall inside the range, a muted
  "Recording since HH:MM" at that point says why the line is short.
- Tests first (web_tests.rs): both add buttons in the page; main.js wires `#deskAdd` to the
  Desktop first-time flow; chart.js has the "Recording since" note and no longer hides the
  figures behind "Not enough figures in this range yet".
- The chart opens on 5h. The range picked is remembered under `cud-range-v2` (a choice
  saved under the old key, from when 24h was the default, no longer applies).

## "Not started yet" and per-model weekly limits (Fable)

- A window with no reset time (Anthropic's `resets_at: null`: no use yet, so the window has
  not begun) reads "0% … · not started yet": dashboard bars (`bar` in base.js) and the
  widget's in-use card (where the time left would be).
- Anthropic's `limits` entries of kind `weekly_scoped` (a weekly limit of one model, named
  by `scope.model.display_name`, e.g. "Fable") are kept as `Usage.scoped` (name, %, reset;
  `to_scoped` in util.rs; absent in older saved state = none). The dashboard shows each as
  a small chip under the bars: "Fable 0%", coloured by level, reset in its tooltip.
- Tests first: `to_scoped` (one Fable entry kept, other kinds ignored, no `limits` = none);
  web: base.js says "not started yet", rows.js draws the `scoped` chips.

## Auto-switch: one threshold per window

1% of the weekly limit is about 5× the use of 1% of the 5-hour limit, so the weekly one
switches later: **5-hour at 95%** (`threshold`, as before), **weekly at 99%**
(`weekThreshold`, new; saved settings without it read 99).

- The account in use is moved when its 5-hour % ≥ `threshold` or its weekly % ≥
  `weekThreshold`; an account it may move to is under both (`switch_reason`, `has_room`).
- Settings: "Auto-switch at [95] % 5-hour · [99] % weekly". The chart draws each window's
  own dashed line in its colour; the legend reads "95% / 99% auto-switch".
- Tests first: weekly 96% stays (was a switch at 95), weekly 99% switches, 5-hour 95%
  switches; an account at weekly 97% can be moved to; old settings read weekThreshold 99.
