# Spec: per-account "Check now"

## Why
When an account's figures look stale, the user wants to see *that account's* real
remaining quota at once, without waiting for its scheduled check. The earlier
top-bar "Check now" forced a poll of every account; the user asked instead for a
button on each account that checks only that account, right away.

## What
- A **Check now** button on each account card (only where there is a Claude CLI
  account to poll — Desktop-only rows have no `id`).
- Pressing it forces an immediate usage read of that one account and redraws the
  card with the fresh figures (or its error, unchanged behaviour on failure).

## Input / Output
- Endpoint: `POST /api/accounts/<id>/check` (page-only write: needs `X-CUD: 1`
  and a loopback Host, like every other write).
- Effect: `poll_account(id, force = true)` — bypasses the next-check schedule,
  reads usage now (with the Desktop-token fallback already in place), saves state.
- Returns: `200` + `public_state()` (the whole state, so the page redraws), or
  `400 {"error":"not found"}` for an unknown id.

## Done when
- `cargo test` green, including a web test asserting the per-account button and
  its `/api/accounts/<id>/check` wiring.
- No source file over 200 lines.

## Known limit (unchanged by this button)
A parked account whose stored refresh token was spent (rotation) can't be read live
until it is signed in again. "Check now" forces the attempt; it does not invent a token,
and never borrows Claude Desktop's (see [no-foreign-credentials.md](no-foreign-credentials.md)).
