# Claude Desktop: every account's Code sessions (spec)

Why and how others do it: `docs/SO_SANH_DESKTOP_SESSIONS.md`. Claude Desktop lists Code
sessions per account from `<desktop dir>/claude-code-sessions/<account>/<org>/local_*.json`
("cards"; the transcripts in ~/.claude/projects are shared). After a Desktop switch the
other account's sidebar is empty. This shares the cards between all accounts.

## When

- Inside the Desktop switch (desktop/switch.rs), after Claude Desktop has closed and its
  sign-in was saved, before it starts again; also for a switch to "signed out" (add an
  account). Desktop is not running then, so every folder can be written.
- Only when the setting `shareDesktopSessions` is on (default true).
- A failure never fails the switch: the event log says
  "Claude Desktop: Code sessions not shared: <error>" and the app starts as usual.
- Done: "Claude Desktop: Code sessions shared (A added, U updated, D removed)" when
  anything changed; nothing logged otherwise.

## What

- Folders: `<account>/<org>` under `claude-code-sessions` whose names are both UUIDs.
  Fewer than two: nothing to do. Only `local_*.json` cards and `archived-sessions.idx` are
  written; every other file there (backlog, waiting-input, scheduled-tasks.json) is left
  alone.
- A card that is not a JSON object with a string `sessionId` is never copied, changed or
  removed.
- Freshness of a card: max(`lastActivityAt`, `lastFocusedAt`), 0 when neither.
- Account keys (stay with the folder, never copied across): `remoteMcpServersConfig`,
  `enabledMcpTools`, `toolSurfaceSnapshot`, `bridgeSessionIds`,
  `remoteControlAutoEligible`, `steeredByRemoteClient`, `error`, `errorAt`,
  `priorErrorMark`.
- Add: a folder without a card gets the freshest card (ties: the folder key) minus the
  account keys, plus `remoteMcpServersConfig`, `enabledMcpTools`, `toolSurfaceSnapshot`
  taken from that folder's own freshest card that this app did not copy there (else from
  another folder of the same account; else none).
- Update (changed since the last sync, 2026-11-12; prior art docs/SO_SANH_SESSION_3WAY.md):
  - A card's mark: FNV-1a 64 of its JSON without the account keys, keys sorted at every
    level (Desktop's own key order does not count). The state keeps each folder's marks as
    they were after the last sync.
  - Changed copies: those whose mark differs from their folder's kept mark, or with no
    kept mark (first sync, new folder). None changed: every copy is a candidate.
  - Winner: of the candidates, the highest (freshness, file modified time, folder key).
    So an archive, rename or other change that leaves lastActivityAt and lastFocusedAt as
    they were still wins when it is the only change; when two folders changed the same
    card, the later activity wins (then the later write).
  - Every other copy whose mark differs from the winner's gets the winner's keys except
    the account keys, which it keeps. Same mark: left.
  - Kept marks after the pass: what each folder holds then, so a write that failed is not
    taken for a change at the next sync. Except the winner of a card not copied
    everywhere: it keeps its old mark (none: none), so it still counts as changed and wins
    again, whatever the file times (as Unison and rclone bisync save only finished copies).
  - Two folders changing the same card between syncs is near impossible (one account runs
    Desktop at a time); card-level, the later activity wins and the other is overwritten.
  - Backup: also before the first sync that keeps marks (state without `marks`), since it
    settles every card that differs (measured on a copy of the real folders: 278 updates,
    then 0 at the next sync, no card's times moved back).
  - Bug it fixes (measured 2026-11-12): archiving does not move the times, so one account
    showed 116 sessions in its sidebar while the others showed 26.
- Remove: a card this app saw in a folder at the last sync and that is gone now was
  deleted by the user there: the copies in the other folders are moved to
  `<data dir>/desktop-sessions/trash/<account8>-<org8>/<name>.<ms>` (not deleted). A
  folder that cannot be read is not "everything deleted".
- Archive index: each folder's `archived-sessions.idx` becomes
  `{"v":1,"archived":[sessionId of every card with isArchived true, sorted]}`, written only
  when it differs.
- Writes go through a temp file in the same folder, then rename.
- State: `<data dir>/desktop-sessions/state.json`:
  `{"seen": {"<account>/<org>": [names]}, "copied": {"<account>/<org>": [names]},
  "marks": {"<account>/<org>": {"<name>": "<16 hex>"}}, "backedUp": true}`, saved after
  each sync.
- Backup: before the first sync ever, the whole `claude-code-sessions` folder is copied to
  `<data dir>/desktop-sessions/backup-<yyyymmdd-hhmmss>/`.
- Not in the demo.

## Page

- Settings card, a checkbox under "Auto-switch": "Show every account's Code sessions in
  Claude Desktop (when switching Desktop)". `POST /api/settings {shareDesktopSessions}`.
- Help: one short answer: switching Claude Desktop now copies the Code session list so
  every account shows the same sessions; the conversations themselves were never lost;
  the first message after a switch rebuilds its context, which uses some quota; do not
  switch while a reply is running.

## Test first (temp folders, no real Desktop)

- Two accounts, a card only in A: B gets it without the account keys, with B's own
  connector keys from a card native to B.
- Fresher card in B: A's copy takes its fields, keeps A's account keys; same mark: untouched.
- Equal freshness, different cards, first sync: the later-written file wins.
- Archived in A only since the last sync (times unchanged): B gets isArchived and its index.
- Changed in both: the later activity wins. A failed write is not a change next time; an
  archive not copied everywhere still wins next time, whatever the file times.
- The first sync that keeps marks backs up again, once.
- The mark ignores key order and the account keys.
- A card deleted in A since the last sync: B's copy goes to trash; a folder that cannot
  be read deletes nothing.
- Unreadable card: untouched everywhere and never copied.
- Archive index rewritten from isArchived; other files untouched.
- First sync makes the backup; the second does not.
- Setting off: switch does no sync; a sync error still starts the app (switch test with
  the fake app).
