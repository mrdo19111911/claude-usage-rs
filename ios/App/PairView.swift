// Pairing: scan the QR the PC's dashboard shows, or paste its text
// `scas://pair?host=..&port=..&token=..`.
import SwiftUI
import SCASKit

struct PairView: View {
    @EnvironmentObject private var model: AppModel
    @State private var text = ""
    @State private var scanning = false
    @State private var wrong = false

    var body: some View {
        Form {
            Section {
                Text("Open the SCAS dashboard on your PC and turn on Phone access. "
                     + "It shows a QR code and a pairing text. The iPhone must be on the same Wi-Fi, "
                     + "or both on Tailscale.")
                    .font(.callout)
            }
            Section("Scan") {
                Button { scanning = true } label: { Label("Scan the QR code", systemImage: "qrcode.viewfinder") }
                    .disabled(!QRScanner.available)
                if !QRScanner.available {
                    Text("No camera here: paste the text instead.").font(.caption).foregroundStyle(.secondary)
                }
            }
            Section("Or paste the pairing text") {
                TextField("scas://pair?host=…&port=…&token=…", text: $text, axis: .vertical)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.footnote.monospaced())
                Button("Pair") { submit(text) }
                    .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                if wrong {
                    Text("That is not a SCAS pairing text.").font(.caption).foregroundStyle(.red)
                }
            }
        }
        .navigationTitle("Pair with your PC")
        .sheet(isPresented: $scanning) {
            QRScanner { code in
                scanning = false
                submit(code)
            }
            .ignoresSafeArea()
        }
    }

    private func submit(_ value: String) {
        wrong = !model.pair(value)
    }
}
