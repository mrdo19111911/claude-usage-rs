// swift-tools-version: 5.9
// SCASKit: the phone's logic (feed v1, pairing, texts, split, client, store), no UI.
// Spec: docs/specs/ios.md. Tested with `swift test` on macOS (CI: .github/workflows/ios.yml).
import PackageDescription

let package = Package(
    name: "SCASKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "SCASKit", targets: ["SCASKit"]),
    ],
    targets: [
        .target(name: "SCASKit"),
        .testTarget(name: "SCASKitTests", dependencies: ["SCASKit"]),
    ]
)
