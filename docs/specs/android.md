# Android (spec)

How it is built: `docs/specs/android-prior-art.md`. Code: `android/scas-companion/`.

## What a phone can and cannot do

- No Claude Code CLI and no `~/.claude` on a phone, so there is nothing to switch there.
  The switching, the saved logins and the polling stay on the PC.
- SCAS's sign-in reuses Claude Code's OAuth client (`anthropic.rs` `CLIENT_ID`) with a
  `http://localhost:<port>/callback` return. A phone app doing the same would be a second,
  unknown program holding Claude tokens on a new device: exactly what Anthropic flagged
  as infostealer-like on 2026-11-12 (`no-foreign-credentials.md`).

## The three options

| Option | What it is | Verdict |
|---|---|---|
| (a) Companion | An app that opens the dashboard of the SCAS running on the user's PC | **Chosen** |
| (b) Standalone viewer | Own OAuth sign-in on the phone (PKCE as `signin.rs`), calls `/api/oauth/usage` itself | Rejected: Claude tokens on another device, under Claude Code's client id, polled from a phone IP; a new copy of every account's login; no switching anyway |
| (c) Rust server in the app | SCAS's core built with the NDK, served to a WebView inside the app | Rejected: same token problem as (b), plus the NDK, and most of the core (creds.rs, desktop/, switch.rs) has nothing to act on |

Why (a): no token ever leaves the PC; the dashboard (`src/web/`) is reused as it is, so
the phone shows the same figures, charts and buttons (Use here switches Claude Code on the
PC); the app holds no secrets, only an address.

## Reaching the PC

SCAS listens on `127.0.0.1` only and refuses any Host but `127.0.0.1:P` / `localhost:P`
(`server.rs`, DNS-rebinding guard). The companion therefore works today, with no change on
the PC, through a tunnel that ends at the PC's localhost:

1. USB: `adb reverse tcp:P tcp:P` (P = SCAS's port; macOS/Linux 4777, Windows the
   number in `~/.claude-usage-dashboard/port`). The app opens `http://127.0.0.1:P`.
2. Any SSH tunnel to the PC (e.g. Termux `ssh -L P:127.0.0.1:P pc`), same address.

A LAN or Tailscale address is refused by SCAS (403 "forbidden host"). The network way in
is the shared mobile feed (`docs/specs/mobile-feed.md`, built by the iOS session in
`src/mobile/`, not switched on in main yet): an opt-in read-only listener,
`GET /api/mobile/v1/state` with `Authorization: Bearer <token>`, paired with
`scas://pair?host=..&port=..&token=..`; its "Stable contract" section is what Android
reads. Android's pairing mode (next step, not in this MVP): paste or scan that text,
then draw a native read-only view from the JSON.

Feed v1 is plain HTTP to private/Tailscale IPs. `network_security_config` can allow
cleartext per domain or exact IP only, not per range, and it is fixed at build time,
so it cannot list a host paired later. The pairing mode will therefore set
`cleartextTrafficPermitted="true"` in `base-config` and refuse in code (`Address`) any
http address that is not 127/8, 10/8, 172.16/12, 192.168/16, 100.64/10 or fd00::/8.

## The app (MVP)

- One screen. First run, or no address saved: an address box (default
  `http://127.0.0.1:4777`) and **Open**. Then a full-screen WebView on that address.
- The address is cleaned by `Address.normalize` (pure Java, unit-tested):
  - trims spaces; `4777` alone or `:4777` → `http://127.0.0.1:4777`;
  - no scheme → `http://`; only `http` and `https` kept, anything else rejected (null);
  - a host is required; no port → `4777`; the path is dropped (the dashboard is `/`).
  - `Address.passesHostCheck(url)` is true for `127.0.0.1` / `localhost` only; for other
    hosts the screen says SCAS will refuse it (see "Reaching the PC").
- WebView: JavaScript and DOM storage on (the page keeps its range choice in
  localStorage); cleartext HTTP allowed only for `127.0.0.1` and `localhost`
  (`network_security_config.xml`); links to other hosts (release page, claude.ai) open
  in the phone's browser; a load error shows the address box again with the error.
- No menu: the system Back key at the dashboard's root goes back to the address box,
  so the address can be changed.
- No permissions but `INTERNET`. No tokens, no accounts, nothing stored but the address
  (SharedPreferences `scas`, key `url`).
- minSdk 24, targetSdk 35, package `vn.smartlog.scas.companion`, debug-signed.

## Build (no Gradle, no NDK)

`android/scas-companion/build.ps1`: aapt2 compile + link (manifest, res, android.jar) →
javac (release 11) → d8 → add `classes.dex` → zipalign → apksigner with a debug key made
by keytool. Output: `target/android/scas-companion.apk`. Needs `ANDROID_HOME` with
`build-tools;35.0.0` and `platforms;android-35` (steps in `android/README.md`).

## Tests

- `android/scas-companion/test/AddressTest.java`: plain `java` runner, no JUnit (none on
  this PC), exits 1 on a failure. Run: `android/scas-companion/test.ps1`. Written first.
- On a phone (manual, after the SDK is installed): `adb install`, `adb reverse`, the
  dashboard loads and Use here switches the PC's Claude Code.
