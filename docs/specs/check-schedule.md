# Check schedule

When the app asks Anthropic for an account's usage next (`next_check_at`, src/schedule.rs).

## Why

Anthropic's usage endpoint refuses with 429 when asked too often. On 2026-11-06 two logins
were refused while checked every minute. The old rule asked every 2 min, and up to every
15 s near a limit, for every account. It used the larger of the 5-hour and weekly %, so a
week at 72% meant a check every minute even with the 5-hour window at 34%.

Measured on the 7-day usage log (16,028 checks, 6 accounts):
- The 5-hour % rose at most 11 points in 5 min and 20 in 10 min (about 2%/min).
- From 90% to 100% took 12 min at the fastest.

Replay of that log (scratch `sim.py`):

| Schedule | checks/h | % when a 95% crossing is seen (median / worst) |
|---|---|---|
| old | 202 | 96 / 98 |
| this one, without the idle rule | 41 | 96 / 98 |
| this one, with idle accounts every 10 min | 28 | 97 / 99 |

## Rule

- **At 100% with a known reset still ahead** (either window): no check until that reset,
  plus 1 min. Unchanged from the old rule.
- **In use**: the Claude CLI login in use, or the account Claude Desktop is signed in to.
  Its nearness to a limit, in 5-hour points:
  `p = max(5-hour %, 100 − (100 − weekly %) × K)`, with K = 6.7 (1% of a week ≈ 6.7% of
  5 hours). The interval falls linearly from 10 min at p = 0 to 1 min at p = 90, and stays
  at 1 min above that.
- **Not in use**: every 10 min. Its % only moves when it is used somewhere else.
- **After an error** (other than 429): again in 2 min. After a 429, the existing backoff
  applies: 5, 10, 20, then 40 min, or the wait Anthropic names.
- **On a change of use** (an account starts or stops being used): its next check is worked
  out again from its last figures with the new rule. One just switched to is therefore due
  at once, unless it is at 100% and waiting for its reset. Its effective % is used: a
  window whose reset has passed counts as 0.
- **Refresh now** still checks every account at once.

## Settings

"Check each account every N s" (`pollSeconds`) goes from the page, because the rule above
sets the schedule. The field stays in saved state files so that old ones still load; it is
no longer read or set.

## Tests first (src/schedule_tests.rs)

- Interval in use: 10 min at 0%, 5.5 min at 45%, 1 min at 90% and at 97%.
- Weekly counts as 5-hour points: a week at 72% with the 5-hour window at 0% gives 10 min;
  a week at 97% (79.9 points) gives about 2 min; a week at 99% gives 1 min.
- Not in use: 10 min at any %.
- Full with a known future reset: the reset plus 1 min, in use or not. Full with an unknown
  or past reset: 1 min in use, 10 min not.
- `mark_in_use`: switching to an account makes it due at once; one that stops being used
  moves to 10 min after its last check; an account at 100% keeps waiting for its reset; an
  account with an error keeps its retry time.

## Parked accounts whose token has run out
Their sign-in is refreshed only when an account in use nears a limit or on Check now / Refresh now; until then they wait with their last figures. See [idle-refresh.md](idle-refresh.md).
