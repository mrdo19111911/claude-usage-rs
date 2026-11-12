# Spec: renewing saved Claude Desktop sign-ins at 4:00 (asked 2026-11-12)

## Why
claude.ai keeps a sign-in (`sessionKey`) 28 days from its last use; a saved Desktop sign-in
not opened for 28 days ends (docs/specs/desktop-signed-out.md). The user does not want to
watch dates: "tự gia hạn 1 lần vào 4h sáng là đủ … sau đó sẽ chuyển ngược lại acc cũ".

## Rule (`src/desktop_renew.rs`, tests `desktop_renew_tests.rs`)
`next_step(on, state, signed_in, desk, until, code_running, now) -> Step`, pure:
- A renewal under way (`state.renewing = {target, back, at}`):
  - less than `STAY_MS` (3 min) since `at`: `Wait` (Desktop opens claude.ai on the target,
    which issues a new 28-day `sessionKey`);
  - then, Desktop signed in to `back`: `Done`; otherwise `Back(back)` (also when the setting
    was turned off meanwhile).
- None under way: `Renew(uuid)` when all hold, else `Wait`:
  - the setting `renewDesktop` is on (default on);
  - the local hour is `RENEW_HOUR` (4, i.e. 4:00-4:59);
  - Desktop is signed in to an account (the one to come back to);
  - a saved account other than that one is due: its saved sign-in is live and ends within
    `RENEW_WITHIN` (7 days); not one tried already today (`state.day`, `state.tried`);
    the one ending first;
  - no Claude Code session runs in Desktop (Desktop ends idle ones after 30 min), unless
    the due sign-in ends within `MUST_WITHIN` (2 days): then it is renewed anyway.
  Ended (`Session::Ended`) or unknown sign-ins are not due: they need a sign-in by hand.

## App (glue in `desktop_renew.rs`, each poll after `desktop_auto`)
- The renew state lives in state.json (`desktopRenew`), so a restart mid-renewal still
  switches back.
- `Renew`: log "Claude Desktop: renewing <target>'s saved sign-in (N days left)", add it to
  `tried`, store `renewing`, `start_desktop_switch(target)`.
- `Back`: `start_desktop_switch(back)`; Desktop auto-switch waits `GAP_MS` from then.
- `Done`: clear `renewing`; log "<target>'s sign-in renewed (N days left)" when it now ends
  20+ days away, else "<target>'s sign-in was not renewed (N days left)".
- Desktop auto-switch does nothing while a renewal is under way.
- Several due accounts: one per poll cycle, each Renew → Done before the next.

## Page (tests `web_desktop_renew_tests.rs`)
- Settings: the checkbox "Renew saved Claude Desktop sign-ins at 4:00 (switches Desktop for a few
  minutes, then back)" under the Code sessions one; shows `settings.renewDesktop` (missing = on);
  a change is sent to POST /api/settings with the other settings.
- The "Sign-in until" tooltip says one due within 7 days is renewed at 4:00.
- Quiet polls (no account due for a check) still run the rule, so 4:00 is never missed.

## Done when
- Tests: every branch above; the settings route and page carry `renewDesktop`.
- On this computer: the rule run against the real saved profiles at a fake 4:00 picks none
  today (all end 2026-12-09 or later) and picks the earliest one 7 days before it ends.

Prior art: docs/SO_SANH_DESKTOP_RENEW.md.
