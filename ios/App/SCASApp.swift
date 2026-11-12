// SCAS for iPhone: read-only companion of the PC's SCAS (docs/specs/ios.md).
// Not paired: the pairing screen. Paired: the accounts screen, refreshed on open and on return.
import SwiftUI
import SCASKit

@main
struct SCASApp: App {
    @StateObject private var model = AppModel()
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        WindowGroup {
            NavigationStack {
                if model.pairing == nil {
                    PairView()
                } else {
                    AccountsView()
                }
            }
            .environmentObject(model)
            .task { await model.refresh() }
            .onChange(of: phase) { _, now in
                if now == .active { Task { await model.refresh() } }
            }
            .onOpenURL { url in
                // A scas://pair link opened from the camera app or a message.
                _ = model.pair(url.absoluteString)
            }
        }
    }
}
