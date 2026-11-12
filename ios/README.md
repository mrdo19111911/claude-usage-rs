# SCAS for iPhone

Read-only companion of SCAS on the PC: shows each Claude account's 5-hour and weekly
limits, which account Claude Code / Claude Desktop use now, and when each resets, read from
the PC's mobile feed over the same Wi-Fi or Tailscale. Spec: `docs/specs/ios.md`; feed:
`docs/specs/mobile-feed.md`.

## Layout

| Path | What |
|---|---|
| `SCASKit/` | Swift package with all the logic (feed v1, pairing, texts, split, client, store) and its XCTest tests |
| `App/` | SwiftUI app: pairing screen (paste or QR scan), accounts screen |
| `Widget/` | WidgetKit extension, small and medium |
| `project.yml` | XcodeGen spec; it writes `SCAS.xcodeproj`, both `Info.plist` and both `.entitlements` |

## Build on a Mac

Needs macOS with Xcode 16 or newer.

```sh
brew install xcodegen
cd ios
xcodegen generate
open SCAS.xcodeproj
```

- Tests of the logic, no Xcode project needed: `cd ios/SCASKit && swift test`.
- Simulator: pick an iPhone simulator, Run. No Apple account needed.
- Your iPhone: select the `SCAS` target, Signing & Capabilities, pick your Team (a free
  Apple ID works, see below); do the same for `SCASWidget`. Both keep the App Group
  `group.com.smartlog.scas`. If the bundle id `com.smartlog.scas` is taken for your team,
  change `PRODUCT_BUNDLE_IDENTIFIER` for both targets in `project.yml` (the widget's must start
  with the app's) and the group name in `project.yml` and `SCASKit/Sources/SCASKit/Store.swift`,
  then `xcodegen generate` again.
- Run `xcodegen generate` again after adding or removing files; do not edit the
  `.xcodeproj` by hand (it is not kept in git).

CI does the same on GitHub's `macos-latest`: `.github/workflows/ios.yml` runs `swift test`,
generates the project, builds for the simulator with signing off and uploads
`SCAS-simulator.zip` (drag the unzipped `SCAS.app` onto a booted simulator to install).

## Apple accounts

| Want | Needs |
|---|---|
| Simulator | Nothing |
| Your own iPhone, up to 7 days per install | Free Apple ID as a personal team (App Groups: check at the first device build) |
| Your iPhone beyond 7 days, App Groups for sure | Paid Apple Developer Program ($99/year) |
| TestFlight, App Store | Paid Apple Developer Program |

## Pairing

On the PC, turn on Phone access in the SCAS dashboard; it shows a QR code and the text
`scas://pair?host=..&port=..&token=..`. Scan it or paste it in the app. The token goes to
the Keychain, host and port to the App Group so the widget can read them. "Forget this PC"
(menu at the top right) removes both and the last figures.

The phone must reach the PC: same Wi-Fi, or both on Tailscale. iOS asks once for Local
Network access; say Allow.
