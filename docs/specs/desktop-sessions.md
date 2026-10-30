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
- Update: a folder whose card is less fresh than the freshest gets the freshest card's
  keys except the account keys, which it keeps from its own card. Same freshness: left.
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
  "backedUp": true}`, saved after each sync.
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
- Fresher card in B: A's copy takes its fields, keeps A's account keys; equal: untouched.
- A card deleted in A since the last sync: B's copy goes to trash; a folder that cannot
  be read deletes nothing.
- Unreadable card: untouched everywhere and never copied.
- Archive index rewritten from isArchived; other files untouched.
- First sync makes the backup; the second does not.
- Setting off: switch does no sync; a sync error still starts the app (switch test with
  the fake app).
