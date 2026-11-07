# Pace line: how other open-source usage trackers project a limit

Read 2026-11-07, for the pace line on the 5-hour chart (docs/specs/account-cards.md).

| Project | Method | Fit for whole-% checks every 1-10 min | Cost to adopt |
|---|---|---|---|
| Maciek-roboblog/Claude-Code-Usage-Monitor (`core/calculations.py`) | Tokens/min since the session block started, plus a rolling last-hour rate; run-out = remaining / rate; a rate of 0 shows the reset time instead | Low: built on token counts, which this app does not have | Low, about 10 lines |
| ryoppippi/ccusage (`rust/crates/ccusage/src/blocks.rs`) | Block tokens / (last entry - first entry); projects the total at the block's end, not a time | Low: tokens, no noise handling | Low, little to reuse |
| steipete/CodexBar (`Sources/CodexBarCore/UsagePace.swift`) | From % directly: rate = % / time since the window started; ETA = (100 - %) / rate; at or past the reset: "Lasts until reset" | High: same inputs (% and reset) | Very low |
| frankchiu-dev/claude-codex-usage-dashboard (`public/forecast.js`) | (last - first) / span over the last N hours, floored at 0; needs 3 samples over a minimum span; drops data from before a reset | High: built for whole-number % | Low, about 30 lines |

Chosen: none of them fit a line; they take first/last over a span. Both kinds (CodexBar's
"since the window started" and frankchiu's "first/last over a recent span") were replayed
on this app's 7-day usage log next to least squares over 10-60 min (table in
account-cards.md). Least squares over the last 20 min had the smallest error 0-45 min
ahead; "since the window started" was as good under 15 min but twice as far off 45-120
min ahead, when use speeds up or slows down. Kept from the others: no line for a rate of
0 or under 2 checks, the window's own checks only (none from before its reset), and
"~N% at reset" in place of a time after the reset (CodexBar's "Lasts until reset").
