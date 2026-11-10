# Reading Claude Desktop's token for usage figures

## Why

The live Claude CLI login's access token lasts ~8 hours and the app never refreshes it
(its refresh token is single-use and belongs to Claude Code). So when the user works in
Claude Desktop and does not run Claude CLI, the CLI token expires and the app loses the
in-use account's figures: the "In use now" panel and its card fall back to the last
stale numbers, and a 5-hour window whose reset has passed shows 0%.

Claude Desktop keeps its own long-lived tokens for the same account, including one for
the Claude CLI OAuth client (`9d1c250a-…`), valid for weeks to a year. We read that token
to ask Anthropic for the figures when, and only when, the CLI token has expired.

## What (inputs → output)

`desktop::desktop_access_token(dir, account_uuid) -> Option<Value>`

- `dir`: Claude Desktop's data folder (the one holding `config.json`), from `find_data_dir`.
- `account_uuid`: the in-use account's Anthropic uuid (`Account::account_uuid`).
- Output: `Some({ "accessToken": <token>, "expiresAt": <ms> })` to feed `get_usage`, or
  `None` when no usable token is found (not Windows, folder gone, nothing decrypts, no
  entry for this account/client/scope, or every match has expired).

### How the token is stored (Windows, Electron safeStorage)

- The AES key is in `Local State` → `os_crypt.encrypted_key`: base64, with a 5-byte
  `DPAPI` prefix, DPAPI-wrapped for this Windows user. Unwrap with `CryptUnprotectData`.
- Each value in `config.json` (`oauth:tokenCacheV2`, then `oauth:tokenCache`) is base64 of
  `v10` + 12-byte nonce + ciphertext + 16-byte GCM tag, AES-256-GCM under that key
  (`BCryptDecrypt`, `ChainingModeGCM`). Same scheme as Chrome's cookie store.
- The decrypted JSON maps a key `acct:<account-uuid>|<client-id>:<device>:<url>:<scopes>`
  to `{ token, refreshToken?, expiresAt (ms), … }`.

### Which entry (token_cache::pick_token)

From both caches, keep entries where the key contains `acct:<account_uuid>|`, the Claude
CLI `client_id`, and `user:inference` or `user:profile`, whose `token` is non-empty and
whose `expiresAt` is in the future. Pick the one with the latest `expiresAt`.

## Rules kept

- Read only. The token is used once to GET `api/oauth/usage`; it is never written back,
  never refreshed, and its refresh token is never touched. The app's "nothing is
  decrypted on a switch" rule is unchanged; this is a separate read path for figures.
- Used only for the live in-use account, and only after the CLI token has expired
  (`poll_account`, the `may_refresh == false` branch). A parked account still refreshes
  its own saved login as before.

## Done when

- `pick_token` unit tests pass: right entry chosen; wrong account, wrong client, missing
  scope, empty token and expired entries skipped; latest expiry wins.
- `aes_gcm::decrypt` passes a NIST AES-256-GCM known-answer vector; a tampered tag → None.
- Live: with the CLI token expired, the in-use account shows real current figures on the
  dashboard (not 0%, not the stale numbers), sourced from the Desktop token.

## Files (each ≤200 lines)

- `src/desktop/token_cache.rs` — `pick_token` (pure) + `token_cache_tests.rs`.
- `src/desktop/aes_gcm.rs` — AES-256-GCM decrypt via BCrypt (Windows) + KAT test.
- `src/desktop/safe_storage.rs` — `app_key` (DPAPI) + `decrypt_v10`.
- `src/desktop/token.rs` — `desktop_access_token` orchestration; non-Windows stub = None.
- Wiring: `anthropic.rs` `desktop_usage`, called from `poll.rs` `poll_account`.
