# Loading state.json without losing it (spec)

## Why
2026-11-12 13:47: state.json held a `null` where SCAS expects a number (`slowUntil`). At start
SCAS could not read it, started with no accounts and saved over the file: 6 accounts became 1.
Only a copy made by hand brought them back. Asked the same day: "Sửa lỗi đó".

## What: `state_load::load(path, stamp) -> Result<Loaded, String>` (`src/state_load.rs`)
- No file: `Loaded::Fresh` (first run).
- The file reads as a `State`: `Loaded::Read(state)`.
- The file reads but is not a `State` (bad JSON, a wrong type, a missing field): it is
  **moved aside** to `state.json.unreadable-<stamp>`, never deleted or written over, and
  `Loaded::Kept { aside, why }` comes back with the reason (serde's message). SCAS starts with
  no accounts; its next save writes a new state.json, and the old one stays next to it.
- The file exists but cannot be read (locked, no permission) or cannot be moved aside:
  `Err(reason)`. SCAS then stops at start with that reason rather than run and save over a
  file it never read.
- `App::load_saved(&mut State)` (same module) does this at start: `Read` replaces the state,
  `Kept` logs an event "Saved accounts could not be read (<why>); the file is kept as <aside>.
  Restore it after fixing it, with SCAS closed.", `Err` prints the reason and exits.

## Tests (written first): `src/state_load_tests.rs`
- no file → Fresh; a saved `State` → Read with the same accounts;
- a file that does not read as a State → Kept: the original bytes are at `aside`, nothing is
  left at `path`, and `why` is not empty; a second call gives Fresh (nothing to overwrite);
- `path` is a folder (exists, cannot be read as a file) → Err, and the folder is untouched.
