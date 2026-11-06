# One card per account, with its charts; auto-switch for Claude Desktop (spec)

Asked 2026-11-06 with a mockup: "Usage over time" showed only a 5-hour view and sat apart
from "Claude accounts"; the weekly limit needs the days left until its reset; combine.

## Page: the "Claude accounts" section

The "Usage over time" section goes. "Claude accounts" holds one card per account, in the
order of /api/state `accounts` (auto-switch's order, docs/specs/auto-switch-deadline.md),
then the Claude Desktop accounts with no Claude CLI login. A card:

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

## Claude Desktop auto-switch (backend)

- `DesktopAccount.auto: bool`, serde default false (Desktop restarts: never on unasked).
  /api/state `desktop.accounts[].auto`. `POST /api/desktop/<uuid>/auto {enabled}`.
- Each poll, after Claude CLI's auto-switch: Claude Desktop's account (signed in now) has
  reached a limit: its effective 5-hour or weekly % is at 100 (Claude has stopped
  answering anyway, so closing it loses no running work). Then switch Desktop to the first
  account in auto-switch's ranking that has `auto` on, a saved Desktop sign-in and is
  useful (rank.rs), via `start_desktop_switch`. Reason in the log:
  "Claude Desktop: auto-switch (5-hour 100%)".
- Not when: setting `autoSwitch` off; a Desktop switch is running; Desktop is not here,
  signed out, or this is the demo; the last Desktop auto-switch was under 10 minutes ago;
  no candidate (logged once, as for the CLI: "Claude Desktop hit <which> but no other
  account has room").
- Tests first (pure): the pick from accounts + Desktop list + signed-in uuid + settings +
  now + last auto time.
