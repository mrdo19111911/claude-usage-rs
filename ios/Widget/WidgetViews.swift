// What the widget draws. Small: the in-use account, two bars, time left. Medium: the same at
// the left and the best "Switch to" account at the right. Stale figures greyed.
import SwiftUI
import WidgetKit
import SCASKit

struct WidgetView: View {
    @Environment(\.widgetFamily) private var family
    let entry: FeedEntry

    var body: some View {
        let split = entry.split
        VStack(alignment: .leading, spacing: 4) {
            if let main = split.main {
                if family == .systemMedium {
                    HStack(alignment: .top, spacing: 12) {
                        Card(title: "In use", account: main, now: entry.now)
                        if let best = split.best {
                            Card(title: "Switch to", account: best, now: entry.now)
                        }
                    }
                } else {
                    Card(title: "In use", account: main, now: entry.now)
                }
            } else {
                Text(message).font(.caption).foregroundStyle(.secondary)
            }
            if entry.loaded.stale, entry.loaded.feed != nil {
                Text(stale).font(.system(size: 9)).foregroundStyle(.secondary).lineLimit(2)
            }
        }
        .opacity(entry.loaded.stale ? 0.6 : 1)
    }

    private var message: String {
        if let e = entry.loaded.error { return e == .notPaired ? "Open SCAS to pair with your PC." : Texts.problem(e) }
        return Texts.notChecked
    }

    private var stale: String {
        guard let seen = entry.loaded.seenAt else { return Texts.unreachable }
        return "Last seen \(Texts.clock(seen, now: entry.now, tz: .current))"
    }
}

/// One account: short name, chips, and the two limits.
struct Card: View {
    let title: String
    let account: Account
    let now: Int64

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title.uppercased()).font(.system(size: 9, weight: .semibold)).foregroundStyle(.secondary)
            HStack(spacing: 3) {
                Text(Texts.shortName(account.email)).font(.caption.bold()).lineLimit(1)
                if account.cli { Tag(text: "CLI", color: .teal) }
                if account.desktop { Tag(text: "Desk", color: .purple) }
            }
            if let u = account.usage {
                Limit(label: "5-hour", window: u.five, now: now)
                Limit(label: "Weekly", window: u.week, now: now)
            } else {
                Text(Texts.notChecked).font(.caption2).foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct Limit: View {
    let label: String
    let window: SCASKit.Window
    let now: Int64

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            HStack {
                Text(label).font(.system(size: 10))
                Spacer()
                Text(Texts.percent(window.u)).font(.system(size: 10, weight: .bold)).monospacedDigit()
            }
            ProgressView(value: Texts.fraction(window.u)).tint(color(window.u))
            Text(window.resetsAt.map { "in \(Texts.left($0 - now)) · \(Texts.clock($0, now: now, tz: .current))" }
                 ?? Texts.notStarted)
                .font(.system(size: 9)).foregroundStyle(.secondary).lineLimit(1).monospacedDigit()
        }
    }

    private func color(_ u: Int) -> Color { u >= 100 ? .red : u >= 80 ? .orange : .green }
}

struct Tag: View {
    let text: String
    let color: Color

    var body: some View {
        Text(text).font(.system(size: 8, weight: .semibold))
            .padding(.horizontal, 3)
            .overlay(Capsule().stroke(color, lineWidth: 1))
            .foregroundStyle(color)
    }
}
