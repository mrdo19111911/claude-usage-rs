# XcodeGen syntax: what `project.yml` follows

Looked at before writing `project.yml` (2026-11-12). Nothing could be compiled on the Windows
machine; CI (`.github/workflows/ios.yml`) is the first real check.

| Source | Method shown | Used for | Fit / cost |
|---|---|---|---|
| XcodeGen `Docs/ProjectSpec.md` (github.com/yonaskolb/XcodeGen) | `packages: Name: path:`; `- package: Name` + `product:`; `entitlements: path + properties`; `info: path + properties`; `type: app-extension`; `- target: X` with `embed` (default true for an app); top-level `schemes:` | All target, package, entitlement and scheme syntax | The reference; no WidgetKit example of its own |
| XcodeGen `InfoPlistGenerator.swift` | Generated Info.plist holds only `CFBundleIdentifier`, `CFBundleExecutable`, `CFBundlePackageType` (APPL / XPC!) plus our `properties` | Why `project.yml` lists versions, display name, `UILaunchScreen`, scene manifest itself | Nothing hidden added; we must list every key |
| techconcepts.org "XcodeGen multi-platform projects" | Widget target with `info: path:` and `properties: NSExtension: NSExtensionPointIdentifier: com.apple.widgetkit-extension`; warns that `info:` without `path:` (XcodeGen 2.45+) leaves the widget with no Info.plist and it fails only at runtime | The widget's `info:` block | Always give `path:`; their note that entitlements are rewritten on each generate is why they are not kept in git |
| Stack Overflow 64199060 (deployment target of a widget in XcodeGen) | `type: app-extension`, widget's own `deploymentTarget` | Per-target `deploymentTarget: "17.0"` | Older style with `INFOPLIST_FILE`; replaced by `info:` |

Choices that follow:

- `GENERATE_INFOPLIST_FILE: NO` on both targets, since `info:` writes the file and sets
  `INFOPLIST_FILE` (Xcode's own generation would otherwise be a second source).
- An explicit `schemes: SCAS` building both targets, so `xcodebuild -scheme SCAS` finds it
  on a fresh checkout without Xcode having opened the project.
- `SWIFT_VERSION: "5.0"` (Swift 5 language mode under Xcode 16), matching the package's
  `swift-tools-version: 5.9`; strict Swift 6 concurrency checks would turn the widget's
  completion-handler provider and the AVFoundation delegate into errors.
