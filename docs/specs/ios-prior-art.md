# iOS: prior art and choice (2026-11-12)

Two questions: (1) where the phone gets its figures, (2) how the app is built.

## 1. Where the figures come from

| Approach | Seen in | Fit for SCAS | Cost / risk |
|---|---|---|---|
| Phone signs in itself (Claude Code's OAuth client, PKCE, `code=true` paste-code redirect `https://platform.claude.com/oauth/code/callback`) and calls `/api/oauth/usage` | coqu write-up of Claude Code's OAuth; claude-code issues #45783, #36922, #88877 (redirect URI refused, Aug 2026 still open) | Works away from the PC; one more session per account | Claude Code's client_id used from a phone app: the pattern Anthropic flagged as infostealer-like on 2026-11-12 (docs/specs/no-foreign-credentials.md); the paste-code redirect is reported broken; refresh tokens to keep in the Keychain |
| Desktop app syncs a snapshot; phone only shows it | ClaudeMeter (tartinerlabs, iOS widgets fed by a "Mac-synced snapshot"), CodexBar (iOS companion synced from the Mac via iCloud) | SCAS already polls every account; the phone needs no Claude login at all | Phone sees fresh figures only when it can reach the PC (same Wi-Fi or Tailscale, already installed here); iCloud is not available to a Windows app, so the link is direct HTTP |
| Run the whole Rust core on the phone (server + WKWebView) | Tauri 2 mobile apps | Little: no `~/.claude`, no Claude Code to switch, iOS suspends background servers | Biggest port; still needs approach 1 for tokens |

**Choice: desktop feed.** Same as the two open-source iOS Claude-usage projects found;
no new credentials anywhere; the Android app can read the same feed.

## 2. How the app is built

| Approach | Method | Fit | Cost |
|---|---|---|---|
| Tauri 2 mobile | Rust + web UI in WKWebView, `tauri ios build`; CI on macOS runner can make an unsigned simulator build | Could reuse `src/web/` pages | Widgets (WidgetKit) are a separate Swift target Tauri does not generate; mobile CI marked experimental; pulls the desktop-only parts of the dashboard |
| `cargo-lipo` / `cargo-xcode` + SwiftUI | Rust static lib linked into an Xcode project | Good when real logic lives in Rust | `cargo-lipo` is unmaintained (xcframework replaced fat libs); C toolchain for `ring`/`rusqlite` on iOS |
| UniFFI + XCFramework + SwiftUI | `uniffi-bindgen` Swift bindings, `xcodebuild -create-xcframework`, local Swift package (Mozilla application-services / Firefox iOS do this) | Best Rust-on-iOS route, shares bindings with Kotlin | Worth it only if the phone runs Rust logic; with a feed the phone logic is JSON decode + text |
| Plain SwiftUI + WidgetKit, project from XcodeGen | Swift package `SCASKit` (model, texts, tests with `swift test`) + app + widget, `project.yml` → `.xcodeproj` on CI | Thin client: decode feed, draw bars, widget | No Rust on the phone; logic tested by `swift test` on the macOS runner, not on Windows |

**Choice: plain SwiftUI + WidgetKit, XcodeGen project.** With the feed design the phone
has no logic worth a Rust bridge; UniFFI is the route if that changes (for example if
the phone ever signs in itself). The Rust part lives on the PC: `src/mobile/`.

## Sources

- ClaudeMeter: https://github.com/tartinerlabs/ClaudeMeter
- CodexBar: https://github.com/o1xhack/CodexBar
- UniFFI Xcode guide: https://mozilla.github.io/uniffi-rs/latest/swift/xcode.html
- Mozilla Swift package design: https://mozilla.github.io/application-services/book/design/swift-package-manager.html
- XCFramework in GitHub Actions (zip before upload): https://danielsaidi.com/blog/2025/11/09/building-closed-source-binaries-with-github-actions
- Tauri iOS on CI (unsigned simulator default): https://v2.tauri.app/release/tauri-cli/v2.8.4/
- Claude Code OAuth paste-code flow: https://flopsstuff.github.io/coqu/claude-oauth ; https://github.com/anthropics/claude-code/issues/39072
