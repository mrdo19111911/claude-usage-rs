// The app's state: pairing, the last load, refresh, pair, forget. Logic lives in SCASKit;
// this only holds results for the views and tells the widget to reload after a refresh.
import Foundation
import SCASKit
import WidgetKit

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var pairing: Pairing?
    @Published private(set) var loaded: Loaded
    @Published private(set) var loading = false

    private let store: Store
    private let client: FeedSource

    init(store: Store = .shared(), client: FeedSource = FeedClient()) {
        self.store = store
        self.client = client
        self.pairing = store.pairing()
        self.loaded = Loader.stored(store)
    }

    /// The figures split for the accounts screen.
    var split: Split { Split.of(loaded.feed?.accounts ?? []) }

    func refresh() async {
        guard pairing != nil, !loading else { return }
        loading = true
        loaded = await Loader.load(store: store, source: client, now: Texts.nowMs())
        loading = false
        WidgetCenter.shared.reloadAllTimelines()
    }

    /// Reads the pairing text; false when it is not one.
    func pair(_ text: String) -> Bool {
        guard let p = Pairing.parse(text) else { return false }
        store.forget()
        store.save(p)
        pairing = p
        loaded = Loaded(feed: nil, seenAt: nil, error: nil)
        Task { await refresh() }
        return true
    }

    /// "Forget this PC": host, port, token and snapshot go.
    func forget() {
        store.forget()
        pairing = nil
        loaded = Loaded(feed: nil, seenAt: nil, error: .notPaired)
        WidgetCenter.shared.reloadAllTimelines()
    }
}
