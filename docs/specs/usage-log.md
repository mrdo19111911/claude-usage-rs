# Usage log and export (spec)

For field data on quota use (the group's "Scheduling Token Quotas Across Multiple LLM
Subscription Accounts", section 8: window and weekly % with their reset times, per
account, over 1–2 weeks). state.json keeps only 8 days of (time, 5h %, week %) for the
charts and is rewritten at every save, so the long record is a file of its own.

## The file

- `%USERPROFILE%\.claude-usage-dashboard\usage-log.jsonl` (next to state.json), one JSON
  object per line, appended (never rewritten while running). Not in the demo.
- A line after each usage answer from Anthropic (poll.rs), for that account:
  `{"t": ms, "id": "<account id>", "plan": "Max 5x", "active": true,
    "five": {"u": 13, "resetsAt": ms|null}, "week": {"u": 54, "resetsAt": ms|null},
    "scoped": [{"name": "Fable", "u": 40, "resetsAt": ms|null}]}`
  `active`: Claude Code is signed in to this account now (st.active_id).
- Written only when something changed since that account's last line (a %, a reset
  time, active, plan) or 10 minutes have passed since it: the same figures every 15 s add
  nothing, and 30 days stay a few MB.
- No email, no token, no Anthropic `raw` in it: the account is its app id.
- Kept 60 days (at least the 30 a study needs): older lines are dropped once at start
  (the file rewritten then, through a temp file).
- First start with no file: seeded from state.json's history (8 days), those lines with
  `"seed": true`, plan and resets unknown (null), `active` false.
- A write that fails is logged once ("Usage log: could not write …") and does not stop
  polling.

## Export

- `GET /api/usage-log/export` (same Host check as every request; a GET, so a plain link
  downloads it): `Content-Type: application/x-ndjson`,
  `Content-Disposition: attachment; filename="scas-log-<yyyymmdd>.jsonl"`.
- One line per log line, in the shape Claude Code's status line uses:
  `{"ts": "2026-10-30T12:48:12Z", "account": "acc1", "plan": "Max 5x", "active": true,
    "five_hour": {"used_percentage": 13, "resets_at": 1793374199},
    "seven_day": {"used_percentage": 54, "resets_at": 1793851199},
    "model_weekly": [{"name": "Fable", "used_percentage": 40, "resets_at": 1793851199}],
    "seed": false, "source": "scas"}`
  `resets_at` in Unix seconds (as the status line), null when unknown.
- `account`: acc1, acc2… by the order of state.json's accounts (the first added is acc1);
  ids no longer there get the next numbers in the order they first appear in the file.
  Never an email, an id or a name.
- `/api/state` gains `"usageLog": {"lines": n, "since": ms|null, "bytes": n}` for the page.

## Page

- Settings card, a row under "Updates": "**Usage log** · 1,234 lines since 28 Oct" and a
  button "Export" (a link to the export). With no line yet: "Usage log · nothing yet".
- Help: one short answer — what is recorded (the % and reset times, every account, no
  conversation, no email), where the file is, that Export names accounts acc1, acc2….

## Test first

- A line is written for a new answer; the same answer again within 10 minutes writes
  nothing; after 10 minutes, or with a changed % or reset, it does.
- Old state.json history seeds the file once, marked seed; a second start does not.
- Lines older than 60 days are gone after start; newer ones stay, in order.
- Export: acc numbers follow the accounts' order, an unknown id gets the next number,
  no email or id appears, resets in seconds, ts in UTC with Z.
- /api/state carries usageLog; the export answers with the attachment header.
