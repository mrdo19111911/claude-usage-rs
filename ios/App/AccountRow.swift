// One account: email, plan, CLI / Desktop chips, the 5-hour and weekly bars with % and
// "resets in 4h39m · 22:20" (or "not started yet"), scoped chips ("Fable 3%").
import SwiftUI
import SCASKit

struct AccountRow: View {
    let account: Account
    let now: Int64

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(account.email).font(.headline).lineLimit(1).truncationMode(.middle)
                if account.cli { Chip(text: "CLI", color: .teal) }
                if account.desktop { Chip(text: "Desktop", color: .purple) }
                Spacer()
                if let plan = account.plan { Text(plan).font(.caption).foregroundStyle(.secondary) }
            }
            if let u = account.usage {
                LimitBar(label: "5-hour", window: u.five, now: now)
                LimitBar(label: "Weekly", window: u.week, now: now)
                if let scoped = u.scoped, !scoped.isEmpty {
                    HStack { ForEach(scoped, id: \.name) { Chip(text: Texts.chip($0), color: level($0.u)) } }
                }
            } else {
                Text(Texts.notChecked).font(.caption).foregroundStyle(.secondary)
            }
            if let error = account.error {
                Text(error).font(.caption).foregroundStyle(.orange)
            }
        }
        .padding(.vertical, 4)
    }
}

/// "5-hour   9%   resets in 4h39m · 22:20", then the bar.
struct LimitBar: View {
    let label: String
    let window: SCASKit.Window
    let now: Int64

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack {
                Text(label).font(.caption)
                Text(Texts.percent(window.u)).font(.caption.bold()).monospacedDigit()
                Spacer()
                Text(Texts.resets(window.resetsAt, now: now, tz: .current))
                    .font(.caption2).foregroundStyle(.secondary).monospacedDigit()
            }
            ProgressView(value: Texts.fraction(window.u))
                .tint(level(window.u))
        }
    }
}

struct Chip: View {
    let text: String
    let color: Color

    var body: some View {
        Text(text)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 6).padding(.vertical, 2)
            .overlay(Capsule().stroke(color, lineWidth: 1))
            .foregroundStyle(color)
    }
}

/// Bar colour by how much is used.
func level(_ u: Int) -> Color {
    u >= 100 ? .red : u >= 80 ? .orange : .green
}
