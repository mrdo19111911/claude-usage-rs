> **Removed on 2026-11-12** (docs/specs/no-foreign-credentials.md): reading Claude Desktop's
> token looked like infostealer malware to Anthropic. Kept as history only.

# How the app keeps the in-use account's figures when the CLI token expires

## The problem: one account, two logins, two tokens

Signing in with Google creates an OAuth session: an access token (valid a few hours) and
a single-use refresh token. Each app that signs in gets its own session, holds its own
tokens, and refreshes them on its own.

So one Claude account has **two independent logins** on a machine:

| | Claude CLI (terminal, VS Code) | Claude Desktop |
|---|---|---|
| Token store | `~/.claude/.credentials.json` | the app's `config.json`, encrypted |
| Access token lifetime | ~8 hours | long-lived (weeks to ~a year) |
| Who refreshes it | Claude Code, when it next runs | Claude Desktop, continuously |

Claude Usage reads the CLI token only, and never refreshes it: the refresh token is
single-use and belongs to Claude Code, so refreshing it here would log Claude Code out.
That is deliberate ([src/anthropic.rs](../src/anthropic.rs), `fetch_usage`,
`may_refresh == false` for the live login).

The consequence: when the user works in Claude Desktop and does not run Claude CLI, the
CLI access token expires after ~8 hours. Every usage check for the in-use account then
fails with `token expired: Claude Code renews it the next time it runs`. The "In use now"
panel and the account's card fall back to stale numbers, and a 5-hour window whose reset
has since passed shows 0% — which looks like "nothing used" but just means "unknown".

## The finding: Desktop already holds a usable token

Claude Desktop stores tokens for several OAuth clients, **including the Claude CLI client**
(`9d1c250a-…`, the `CLIENT_ID` in [src/anthropic.rs](../src/anthropic.rs)), for the same
account, with the `user:inference` / `user:profile` scopes the usage endpoint needs, and
with a far-off expiry (observed: ~11 months). Because it is not near expiry, the app can
use it as-is — no refresh, so the single-use-refresh-token problem does not arise here.

Verified live against `https://api.anthropic.com/api/oauth/usage`: the Desktop token
returns HTTP 200 with the real current figures, matching claude.ai.

## How Desktop stores the token (Windows, Electron safeStorage)

This is the same scheme Chrome uses for its cookie store:

1. The AES key is in `Local State` → `os_crypt.encrypted_key`: base64, with a 5-byte
   `DPAPI` prefix, then DPAPI-wrapped for the current Windows user. `CryptUnprotectData`
   unwraps it; no password is needed because DPAPI is tied to the signed-in Windows user.
2. Each value in `config.json` (`oauth:tokenCacheV2`, then `oauth:tokenCache`) is base64 of
   `v10` + a 12-byte nonce + ciphertext + a 16-byte GCM tag, AES-256-GCM under that key.
3. The decrypted JSON maps `acct:<account-uuid>|<client-id>:<device>:<url>:<scopes>` to
   `{ token, refreshToken?, expiresAt (ms), … }`.

## The rule this follows

The app uses the Desktop token **only** for the live in-use account, **only** after the
CLI token has expired, and **only** to read usage figures. It never writes the token
back, never refreshes it, and never touches its refresh token. The older rule — the app
decrypts nothing when switching Desktop accounts, it copies the stored values as they are
— is unchanged; this is a separate, read-only path for figures.

Implementation spec: [docs/specs/desktop-token.md](specs/desktop-token.md).
