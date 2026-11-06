# Auto-switch: use the quota that expires first (spec)

Why: the weekly quota left at an account's weekly reset is lost. The old rule
("most room") stays on a fresh account while another one with 34% left resets in 8 hours.
Prior art: `docs/SO_SANH_AUTO_SWITCH.md`. Rule: Earliest Deadline First (EDF): with one
user drawing on several budgets that expire, spending the one that expires first never
loses anything (swap argument) and loses least when demand is short.

## Terms

- `K = 6.7`: 1% of the weekly window is ~6.7x 1% of the 5-hour one (measured 6.72,
  90% interval 6.2-7.4; quota-data/README.md).
- Effective %: a window whose `resetsAt` is at or before now counts as 0 (it has reset;
  the figure is from before).
- Deadline: the weekly `resetsAt` when it is after now; else unknown, ranked last.
- Room now (weekly % units): `min(100 - week, (100 - five) / K)` with effective %.
- Eligible: enabled, no error, usage known, effective 5h < `threshold` and effective
  week < `weekThreshold`.

## Rule (each poll, setting `useSoonestReset` on, default on)

1. Rank the eligible accounts: earliest deadline first (to the minute), then more room now, then the
   account id. "Useful": effective 5h at least 10 points under `threshold` (no switch for
   a sliver, which would switch back minutes later).
2. The account in use is not eligible (it reached a threshold): switch to the first
   ranked useful account, else the first ranked; reason as before (`auto: 5-hour 95%`, `auto: weekly 99%`). None: stay and log
   "<email> hit <which> but no other account has room" once, as before.
3. The account in use is eligible: switch to the first ranked only when
   - its deadline is at least 1 hour before the deadline of the account in use (an
     unknown deadline of the account in use is later than any known one), and
   - it is useful (above).
   Reason: `auto: weekly resets in <h>h, sooner`.
4. Setting off: the old rule (switch only at a threshold, to the most room:
   max `min(100 - five, 100 - week)`).

Flapping: the deadline order is fixed between weekly resets, so the rule switches at most
when a 5-hour window fills or reopens, a weekly window resets, or a threshold is reached.

## Order shown (page lists, chart, tray "Switch to")

The same ranking, top to bottom: the eligible accounts as in step 1; then the enabled
accounts at a threshold, soonest usable again first (the latest reset among the windows
at a threshold; unknown last); then the disabled, errored and unchecked ones. With the
setting off: as before (the saved order). Either way the accounts at 100% of a limit
(effective %) go last, soonest free again first (the latest reset among the windows at
100%; unknown last).

## Page

Settings, under "Auto-switch": checkbox `#useSoonestReset` "Use the account whose weekly
limit resets soonest first (switches before a limit when another account's quota would
expire unused)". `POST /api/settings {useSoonestReset}`. Help: one short answer.

## Measure (tools/replay_switch.py)

Replays the usage log (live or exported) with both rules on the same demand: the rises of
every account's weekly % summed per 10 minutes, served by the simulated account in use,
with 5-hour windows (cap `threshold`/K weekly %, a window opens at its first use and lasts
5 h) and the logged weekly reset times. Prints per rule: demand served, demand unmet,
weekly % lost at resets, switches.

## Test first (pure function, no clock, no network)

- Account in use fresh (week 1%, resets in 131 h) and another with 34% left resetting in
  8 h: switch to the other; old rule: stay.
- Deadlines 30 min apart: no proactive switch.
- Candidate with 5h at 90 (threshold 95): no proactive switch; at 80: switch.
- Account in use at 5h 95%: the earliest deadline among the eligible wins, not the most
  room; none eligible: None and the log line once.
- A window whose reset has passed counts as 0: an account shown at week 100% whose weekly
  reset passed is eligible (unknown deadline, ranked last).
- Disabled, errored, or no usage: never picked.
- Old saved settings without `useSoonestReset`: on.
