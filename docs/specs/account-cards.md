# One card per account, with its charts; auto-switch for Claude Desktop (spec)

Asked 2026-11-06 with a mockup: "Usage over time" showed only a 5-hour view and sat apart
from "Claude accounts"; the weekly limit needs the days left until its reset; combine.

## Page: the "Claude accounts" section

The "Usage over time" section goes. "Claude accounts" holds one card per account, in the
order of /api/state `accounts` (auto-switch's order, docs/specs/auto-switch-deadline.md),
then the Claude Desktop accounts with no Claude CLI login; last, the accounts at 100% of
a limit (5-hour or weekly, a passed reset counting as 0), in /api/state's order. A card:

- Header, left: email (dot when in use anywhere), plan, ✕ (removes the CLI login and the
  saved Desktop sign-in, each only where it is not in use; asks first).
- Header, right: two buttons, "Claude CLI" (green, the CLI tag colour) and "Claude
  Desktop" (purple). Filled with a ✓ when this account is in use there; else a click uses
  it there (CLI: `activate`; Desktop: `dswitch`, or `dfirst` when no Desktop sign-in is
  saved; CLI with no login: `signin`). Desktop button absent when Desktop is not on this
  computer or in the demo; disabled while a Desktop switch runs.
- Under each button a switch: CLI = "Auto-switch may use it" (`data-toggle`, as before);
  Desktop = "Auto-switch may move Claude Desktop here" (`data-dauto`, below). The Desktop
  switch only when a Desktop sign-in is saved.
- Body, left: the 5-hour chart, the last 5 hours to now, 5-hour and weekly lines, the
  threshold lines, hover tooltip; title "5-hour resets in 3h54m (today 22:19)".
- Body, right: the weekly chart: x from the weekly window's start (reset − 7 days) to its
  reset, one tick per day labelled day-month ("3-11"), the weekly line from history, a
  dashed even-pace line from 0% at the start to 100% at the reset, a "now" line with the
  time; title "Weekly resets in 1d 7h38m (Sat 02:00)". Unknown reset: the last 7 days.
- Bottom: the 5-hour and weekly bars (usageBars, as now).
- Errors, "next check" line: under the header, as now.
- Table view (#viewToggle): the card shows a table of the figures instead of the charts.
- Phone width: header buttons wrap under the email; the two charts stack.

## Charts in "In use now"

Asked 2026-11-10 ("trong cái In use now, sao không có đồ thị?", then "cả đồ thị week
nữa"): the two panels at the top showed only the bars, so seeing the chart of the account
in use meant scrolling to its card.

- Each panel (Claude CLI, Claude Desktop) shows its account's two panes as on the card
  (`pane(a, 1, now)`, `pane(a, 2, now)`: 5-hour chart with its pace line, weekly chart,
  each with its bar; weekly with the model chips), in place of the bars.
- Claude Desktop's account is the Claude CLI login with the same uuid as the Desktop
  account signed in. With none (a Desktop-only account: no history is kept for it), or
  no figures yet, or in table view, the panel keeps the bars as before.
- Hover works there as on the cards (crosshair, tooltip, pace tooltip right of "now");
  while the pointer is on a chart neither the panels nor the cards redraw.
- No wider than the screen (asked 2026-11-10 with a phone-width screenshot: the panel ran
  off the right edge, cutting the reset and the %). Measured at 375 px: the panel's column
  was 509 px in a 309 px space, pushed by the switch menu, whose options are long ("email ·
  5h 0% · week 48%"); hiding the charts changed nothing, hiding the menu fixed it. So a
  panel may shrink below its content (`.side { min-width: 0 }`) and the menu shortens its
  text instead.

## Less space: one panel when both use one account; no bar under a chart; short notes

Asked 2026-11-10 ("làm sao để tiết kiệm không gian", then "làm cả 1, 2, 3"). Measured at
1024 px: a panel was 394 px tall (panes 182, of which the chart 70 and the rest its bar,
"57% left · resets …" and the chips; the auto-switch note 39; the switch menu 36), and
with Claude CLI and Claude Desktop on one account the same charts showed three times.

1. Claude CLI and Claude Desktop on the same account (the CLI login and the Desktop
   account signed in share a uuid): one panel across both columns (`.sides.both`), its
   top line half CLI colour, half Desktop colour; the header has both tags and "Both on
   this account"; then the email and the account's panes once; then, side by side (stacked
   on a phone), each app's note and switch menu, each with its small tag. Different
   accounts, or Desktop absent, signed out or in the demo: the two panels as before.
2. Each pane (on the cards and in the panels) drops its bar and the "57% left · resets
   in …" line under the chart: the chart's dot and the reset line show the same. The
   pane's title row says it instead, right of "5-hour" / "Weekly": "57% left · in 1d 14h"
   (the clock time is under the chart's axis), "100% left · not started" with no reset;
   the % left in the bar's colour (green, amber under 30% left, red under 3%). The
   weekly pane keeps the model chips. Desktop-only accounts, with no chart, keep the bars.
3. Each note is one short line, the full text in its tooltip (`title`):
   Claude CLI "Auto-switch at 95% 5-hour · 99% weekly"; Claude Desktop "Auto-switch at
   98% · switching restarts it"; off: "Auto-switch is off" (Desktop: "Auto-switch is off ·
   switching restarts it"). While Desktop switches, the note says so, as before.
4. Asked next ("đúng" to capping the shared panel's charts; "Fable 0% đặt ngang hàng với
   chữ 5-Hour"). Measured: across both columns a chart was 152 px tall at 1024 px (its
   drawing is 300 x 104 and was scaled up 1.5 times). In "In use now" a chart's drawing is
   now as wide as its pane in pixels (`winChart(a, k, now, W)`, W at least 300), so it is
   104 px tall at any width, its text at its real size. Redrawn when the page's width
   changes, watched with a ResizeObserver on `main` (it also sees width changes that are
   not window resizes, such as a scrollbar appearing). Not checked live: the test browser
   pane was hidden, and a hidden page runs neither resize events nor ResizeObserver;
   calling the redraw by hand drew the charts at the new width (440 px, 104 px tall).
6. The cards too (asked next, "làm luôn"): at 1024 px a card's charts were 152 px tall.
   A card pane's width is the grid's column (`#cards` grid-template-columns, as computed)
   less the card's padding and border (30 px), halved less the 14 px gap when the panes
   sit side by side; measured 923 px column, 439.5 px pane. W = 300 stays the least, so
   a narrow card is as before.
5. The model chips ("Fable 0%") sit in the weekly pane's title row, after "Weekly", level
   with "5-hour" when the two panes are side by side; the row wraps when too narrow.

## Pace line on the 5-hour chart

Asked 2026-11-07: a line from the last checks to show when the limit is expected.

- Pace = the slope of a least-squares line through the 5-hour checks of the last 20 min
  (`PACE_MS`) of this window, the figure carried to "now" left out. Needs 2 checks or
  more and a slope above 0; else no line (a slope of 0 would put the limit at infinity).
- No line either when the % has not moved in the last 10 min (2 checks or more, all the
  same): use has paused and a pace from before would be stale. Not "the last 2 checks
  are the same": near a limit the checks are a minute apart and mostly the same, so that
  hid the line 78% of the time in the last 15 min before 95% (59% at 15-45 min); the
  10-min rule hid it 0-4% of the time, with the same error (table below, "LS20, hide if
  flat 10 min": 2.2 / 8.8, 4.9 / 15.4, 13.6 / 35.9).
- Hover right of "now": the tooltip says "Expected limit at 21:20, in 32 min" (or "in
  1h05m"), or "Expected ~91% at reset (23:50)"; the crosshair moves to that time. The
  label at the line's end is short: "~21:20" or "~91%".
- Drawn from the current point at that pace (dotted, the 5-hour colour) to 100%, or to the
  reset if it does not get there first. No line at 100%, with an unknown reset, or on the
  weekly chart.
- Why 20 min and least squares, and why not weekly: replay of the 7-day usage log
  (scratch `proj.py`; checks under docs/specs/check-schedule.md; predicted vs actual
  time the 5-hour % reached 95; |error| median / p90, minutes):

  | Rule | <15 min ahead | 15-45 min | 45-120 min | no line (<15 min) |
  |---|---|---|---|---|
  | last 2 checks | 5.3 / 11.0 | 11.5 / 22.7 | 29.4 / 75.4 | 78% |
  | first/last 20 min | 2.5 / 9.8 | 5.7 / 19.8 | 14.7 / 40.8 | 0% |
  | least squares 20 min | 2.2 / 8.8 | 5.0 / 16.2 | 13.7 / 36.6 | 0% |
  | least squares 60 min | 2.9 / 8.7 | 6.4 / 14.7 | 16.3 / 45.2 | 0% |
  | since the window started | 2.2 / 10.7 | 5.9 / 22.5 | 25.7 / 67.6 | 0% |

  The % is whole numbers: two checks a minute apart mostly differ by 0, so "last 2 checks"
  gives no line most of the time. On the weekly limit (to 99%) no rule did well: 1-4 h
  ahead the best median error was 35 min with no line 45% of the time, and errors of hours
  further out (a 20-min pace does not hold for days). The even-pace line stays the guide
  there.
- Prior art compared in docs/specs/pace-prior-art.md.

## Claude Desktop auto-switch (backend)

- `DesktopAccount.auto: bool`, serde default false (Desktop restarts: never on unasked).
  /api/state `desktop.accounts[].auto`. `POST /api/desktop/<uuid>/auto {enabled}`.
- Each poll, after Claude CLI's auto-switch: Claude Desktop's account (signed in now) is
  at or over `DESKTOP_AT` = 98% of its 5-hour or weekly limit (effective %). Asked
  2026-11-07 ("cứ trên 98% tự chuyển") after Desktop sat at 98% with the CLI already
  moved: it was 100% (Claude has stopped answering then, so closing it loses no running
  work); at 98% a reply under way may be cut off by the restart. Then switch Desktop to the first
  account in auto-switch's ranking that has `auto` on, a saved Desktop sign-in and is
  useful (rank.rs), via `start_desktop_switch`. Reason in the log:
  "Claude Desktop: auto-switch (5-hour 98%)".
- Not when: setting `autoSwitch` off; a Desktop switch is running; Desktop is not here,
  signed out, or this is the demo; the last Desktop auto-switch was under 10 minutes ago;
  no candidate (logged once, as for the CLI: "Claude Desktop hit <which> but no other
  account has room").
- The page says whose thresholds are whose: Settings reads "Auto-switch · Claude CLI at
  [95] % 5-hour [99] % weekly · Claude Desktop at 98% of either (it closes and opens
  again)", and the Claude Desktop panel says when auto-switch moves it (it used to say
  "auto-switch never changes it"). Asked 2026-11-07: without it, Desktop at 98% with the
  CLI moved looked like a broken app.
- Tests first (pure): the pick from accounts + Desktop list + signed-in uuid + settings +
  now + last auto time.
