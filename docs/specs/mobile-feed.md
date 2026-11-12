# Mobile feed: SCAS figures for a phone (spec)

Why: docs/specs/ios.md and docs/specs/ios-prior-art.md. A phone shows the figures the
PC's SCAS already has; it never holds a Claude login. Shared by iOS and Android.

## What the PC serves

- A second listener, **off unless turned on** (wiring below), on `0.0.0.0:<port>`
  (default 4778), separate from the dashboard's `127.0.0.1` one: the dashboard's
  Host check and write API stay exactly as they are.
- One route: `GET /api/mobile/v1/state`. Anything else: 404. Any other method: 405.
- Asked with `Authorization: Bearer <token>`; wrong or missing token: 401.
- Only from this machine, private networks and Tailscale (`peer::allowed`):
  127/8, ::1, 10/8, 172.16/12, 192.168/16, 100.64/10 (Tailscale's CGNAT range),
  fd00::/8 (Tailscale's fd7a:115c:a1e0::/48 is inside). Others: 403 before the token
  is looked at. Port-forwarded from the internet, a request arrives from a public
  address and is refused.
- Read only: no switching, no settings, no sign-in from the phone in v1.

- **Plain HTTP, no TLS** (v1). Only reachable from the addresses above, so phones allow
  cleartext for private/Tailscale hosts only: iOS `NSAllowsLocalNetworking`, Android a
  `network_security_config` that permits cleartext (Android cannot scope cleartext by
  IP range, so it permits it app-wide or for the paired host). The bearer token is then
  visible to anyone on the same Wi-Fi who sniffs; Tailscale traffic is encrypted by
  WireGuard. TLS with a pinned self-signed cert is a possible v2.

## Stable contract (iOS and Android both read it)

- The pairing URI and the JSON field names below do not change within `v: 1`; a change
  bumps `v` and the path (`/api/mobile/v2/state`), and both phone sessions are told.
- Units: times (`now`, `lastPollAt`, `fetchedAt`, `resetsAt`) are Unix epoch
  **milliseconds** (integers). `u` is the percent used, integer 0–100.
  `resetsAt: null` = that window has not started (no use yet): show "not started yet".
  `usage: null` = never checked; `error` is a string or null.
- Full account (used up): `five.u >= 100` or `week.u >= 100`.

## The token and pairing (`pair.rs`)

- 32 random bytes, base64url without padding (43 chars), kept in `mobile-token` in the
  data folder (created on first start of the feed; deleting it unpairs every phone).
- Compared in constant time.
- The pairing text, shown by the dashboard as a QR code and as text to copy:
  `scas://pair?host=<ip>&port=<port>&token=<token>`; `pair::url` builds it,
  `pair::parse` reads it back (the phones implement the same parse).

## The JSON (`feed.rs`, version 1)

Built from `App::public_state()` (a pure function of its JSON, `feed::from_public`):

```json
{ "v": 1, "version": "1.6.1", "now": 0, "lastPollAt": 0, "polling": false,
  "autoSwitch": true, "threshold": 95, "weekThreshold": 99,
  "accounts": [ { "id": "a1", "email": "x@y", "plan": "Max 20x", "enabled": true,
      "cli": true, "desktop": false, "error": null,
      "usage": { "five": {"u": 9, "resetsAt": 0}, "week": {"u": 24, "resetsAt": null},
                 "fetchedAt": 0, "scoped": [ {"name": "Fable", "u": 3, "resetsAt": 0} ] } } ] }
```

- `cli`: the account Claude Code uses now (`activeId`). `desktop`: the account Claude
  Desktop is signed in to now (a `desktop.accounts[]` row with `signedIn`, matched by uuid).
- Accounts keep public_state's order (auto-switch's order).
- Left out on purpose: tokens (public_state has none), `raw`, `history`, `uuid`, file
  paths, `events`, Gemini, update state. A new field is added only with a new `v` or as
  optional; phones ignore fields they do not know.

## Wiring (main session's call, not done here)

- `mod mobile;` is declared; `mobile::serve::start(state_fn, port, token)` is not called
  yet. To turn it on: a setting "Phone access" (off by default) or `SCAS_MOBILE=1`, then
  `mobile::serve::start(Box::new(move || app.public_state()), port, pair::load_or_create(dir)?)`.
- The dashboard then shows the pairing QR **and the same text with a Copy button**
  (pasting works in an emulator / simulator), one per address of this PC (LAN and
  Tailscale).
- Windows Firewall asks once for the new listener (private networks only).

## Tests (written first)

- `feed_tests.rs`: trims raw/history/uuid/events/paths; `cli`/`desktop` flags; order kept;
  account without usage keeps `usage: null`; settings copied.
- `pair_tests.rs`: token length and alphabet; two tokens differ; `url`→`parse` round trip;
  parse rejects other schemes and missing parts; constant-time `same`.
- `peer_tests.rs`: every allowed range at both edges; public v4/v6 refused.
- `serve_tests.rs`: a real listener on a free port: 200 with JSON with the token, 401
  without, 404 other path, 405 POST.
