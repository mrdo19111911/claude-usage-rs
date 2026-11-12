// The accounts screen: "In use now", "Switch to" (most room first), "Used up · available at"
// (soonest free first). Stale figures stay, greyed, under "Last seen HH:MM · PC not reachable".
import SwiftUI
import SCASKit

struct AccountsView: View {
    @EnvironmentObject private var model: AppModel
    @State private var askForget = false

    var body: some View {
        let now = Texts.nowMs()
        let split = model.split
        List {
            if let e = model.loaded.error { problem(e, now: now) }
            if model.loaded.feed == nil && model.loaded.error == nil {
                Section { ProgressView("Asking the PC…") }
            }
            if !split.inUse.isEmpty {
                Section(Texts.inUse) {
                    ForEach(split.inUse) { AccountRow(account: $0, now: now) }
                }
            }
            if !split.switchTo.isEmpty {
                Section(Texts.switchTo) {
                    ForEach(split.switchTo) { AccountRow(account: $0, now: now) }
                }
            }
            if !split.usedUp.isEmpty {
                Section(Texts.usedUp) {
                    ForEach(split.usedUp, id: \.account.id) { UsedUpRow(item: $0, now: now) }
                }
            }
        }
        .opacity(model.loaded.stale ? 0.55 : 1)
        .refreshable { await model.refresh() }
        .navigationTitle("SCAS")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Menu {
                    Button("Forget this PC", role: .destructive) { askForget = true }
                } label: { Image(systemName: "ellipsis.circle") }
            }
        }
        .confirmationDialog("Forget this PC?", isPresented: $askForget, titleVisibility: .visible) {
            Button("Forget this PC", role: .destructive) { model.forget() }
        } message: {
            Text("The pairing and the last figures are removed from this iPhone.")
        }
    }

    private func problem(_ e: FeedError, now: Int64) -> some View {
        var offline = false
        if case .unreachable = e { offline = true }
        let words = offline ? Texts.lastSeen(model.loaded.seenAt, now: now, tz: .current) : Texts.problem(e)
        return Section {
            Label(words, systemImage: offline ? "wifi.slash" : "exclamationmark.triangle")
        }
        .foregroundStyle(.secondary)
    }
}

/// A full account: muted name and when it can be used again.
struct UsedUpRow: View {
    let item: UsedUp
    let now: Int64

    var body: some View {
        HStack {
            Text(Texts.shortName(item.account.email)).foregroundStyle(.secondary)
            Spacer()
            Text(Texts.availableAt(item.freeAt, now: now, tz: .current)).monospacedDigit()
        }
    }
}
