# Never use another app's credentials

Why: on 2026-11-12 Anthropic emailed that activity on a Claude account looked like
infostealer malware and signed sessions out. SCAS did three things that look exactly
like an infostealer from the server's side; this spec removes or closes them.

## Rules

1. **Claude Desktop's tokens are never read.** SCAS does not open Desktop's
   safeStorage (no DPAPI `CryptUnprotectData`, no `oauth:tokenCache*` decryption) and
   never sends a Desktop-issued token from its own process. When the live Claude Code
   token has expired, its figures wait until Claude Code renews it.
   Removed: `desktop/{token,token_cache,safe_storage,aes_gcm}.rs`,
   `Api::desktop_usage`, `poll::wants_desktop_fallback`, specs `desktop-token.md` and
   `desktop-token-fallback.md` (kept only as history in docs/desktop-token.md).
2. **Claude Desktop is never killed during a switch.** Chromium writes cookies to disk
   about every 30 s; a killed Desktop can leave an older `sessionKey`/`sessionKeyV2`
   on disk than the one claude.ai last issued, and saving that copy means a later
   switch puts an already-replaced cookie back. If Desktop does not quit when asked
   (30 s), the switch stops with "Claude Desktop did not close", nothing is saved or
   swapped, and Desktop is left running.
3. **No Claude Code sign-in is made from Desktop's** (the "auto-mint" idea in
   docs/SO_SANH_CLI_TU_DESKTOP.md is dropped).

## Check

`tests/no_foreign_credentials.rs` scans `src/` and fails if `CryptUnprotectData`,
`decrypt_v10`, `safe_storage`, `TerminateProcess` or `desktop_access_token` appear again.
