# SCAS for iPhone (spec)

Why this design: docs/specs/ios-prior-art.md. The PC side: docs/specs/mobile-feed.md.

## What makes sense on an iPhone

An iPhone has no Claude Code CLI, no `~/.claude`, no Claude Desktop to switch, and iOS
suspends apps in the background, so "auto switch" cannot run there. What the user can
use on the phone is **seeing** the figures: how much of each account's 5-hour and weekly
limit is left, which account Claude Code / Claude Desktop use now, when each resets.

Options weighed:

| | Option | Verdict |
|---|---|---|
| a | Companion: reads the PC's SCAS over the LAN / Tailscale | **Chosen** |
| b | Standalone viewer: its own Claude OAuth sign-in, calls the usage endpoint | Not now: uses Claude Code's OAuth client from a phone (the pattern flagged on 2026-11-12, docs/specs/no-foreign-credentials.md); the paste-code redirect is reported broken; one more login per account |
| c | Rust core as a static library in a SwiftUI app with WKWebView | No: the core's job (switching logins on this computer) has nothing to do on iOS |

Reasons for (a): no Claude credentials ever reach the phone; no new traffic to
Anthropic; the figures are the same ones the PC uses for auto-switch; Tailscale (already
on this PC) makes it work away from home; Android reads the same feed.

## The app (v1, read only)

- **Pair**: scan the QR the dashboard shows (or paste its text)
  `scas://pair?host=..&port=..&token=..`. Kept in the Keychain (token) and the App Group
  defaults (host, port), so the widget reads them too. "Forget this PC" removes them.
- **Accounts screen**: "In use now" (CLI and Desktop chips, as the tray widget), then
  "Switch to" (usable accounts, most room first) and "Used up · available at" (soonest
  free first), the same split as the tray widget (docs/specs/modules.md, Widget v2).
  Each row: email, plan, 5-hour and weekly bars with %, time left and local clock time
  ("resets in 4h39m · 22:20"), scoped chips ("Fable 3%"). Pull to refresh.
- **Not reachable**: the last figures stay, greyed, with "Last seen 14:02 · PC not
  reachable (same Wi-Fi or Tailscale needed)".
- **Widget** (small and medium, WidgetKit): the in-use account's two bars and time left;
  medium adds the best account to switch to. Timeline refresh every 15 min (iOS decides
  the real rate); it fetches the feed itself, falling back to the app's last snapshot.
- No switching, no settings, no notifications in v1 (later: v2 of the feed with a
  "switch" call, then a confirm on the phone).

## Code layout (`ios/`)

- `ios/SCASKit/` Swift package (logic, tested with `swift test` on macOS):
  `Feed.swift` (Codable feed v1), `Pairing.swift` (parse the pairing text),
  `Texts.swift` (time left, clock, %), `Split.swift` (in use / switch to / used up),
  `FeedClient.swift` (URLSession GET with the token). Tests in `Tests/SCASKitTests/`.
- `ios/App/` SwiftUI app; `ios/Widget/` widget extension; `ios/project.yml` XcodeGen
  project (bundle id `com.smartlog.scas`, iOS 17).
- Info.plist: `NSLocalNetworkUsageDescription`, `NSCameraUsageDescription` (QR),
  ATS `NSAllowsLocalNetworking` (plain HTTP to LAN and Tailscale addresses only).
- Every file ≤200 lines.

## Build

- Needs a Mac with Xcode 16+ (none here: this machine is Windows). CI:
  `.github/workflows/ios.yml` on `macos-latest`: `swift test` for SCASKit, then XcodeGen
  and `xcodebuild` for the iOS Simulator with signing off, `.app` zipped as an artifact.
- **Needs an Apple developer account** (paid, $99/year): running on a real iPhone beyond
  7 days, App Groups on a device, TestFlight, App Store. A free Apple ID in Xcode can
  install on the user's own iPhone for 7 days (App Groups are allowed with a personal
  team in recent Xcode, but check at first device build).
- Simulator: no account needed.
