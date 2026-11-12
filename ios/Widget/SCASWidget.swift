// The SCAS widget (small, medium): the in-use account's two bars and time left; medium adds
// the best account to switch to. Each timeline fetches the feed itself and falls back to the
// app's last snapshot (App Group); next refresh asked for in 15 min (iOS picks the real time).
import SwiftUI
import WidgetKit
import SCASKit

struct FeedEntry: TimelineEntry {
    let date: Date
    let loaded: Loaded

    var split: Split { Split.of(loaded.feed?.accounts ?? []) }
    var now: Int64 { Texts.nowMs(date) }
}

struct FeedProvider: TimelineProvider {
    static let every: TimeInterval = 15 * 60

    func placeholder(in context: Context) -> FeedEntry {
        FeedEntry(date: Date(), loaded: Loader.stored(Store.shared()))
    }

    func getSnapshot(in context: Context, completion: @escaping (FeedEntry) -> Void) {
        if context.isPreview {
            completion(placeholder(in: context))
            return
        }
        Task {
            let loaded = await Loader.load(store: Store.shared(), source: FeedClient(), now: Texts.nowMs())
            completion(FeedEntry(date: Date(), loaded: loaded))
        }
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<FeedEntry>) -> Void) {
        Task {
            let loaded = await Loader.load(store: Store.shared(), source: FeedClient(), now: Texts.nowMs())
            let now = Date()
            let entry = FeedEntry(date: now, loaded: loaded)
            completion(Timeline(entries: [entry], policy: .after(now.addingTimeInterval(Self.every))))
        }
    }
}

struct SCASWidget: Widget {
    let kind = "SCASWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: FeedProvider()) { entry in
            WidgetView(entry: entry)
                .containerBackground(.fill.tertiary, for: .widget)
        }
        .configurationDisplayName("Claude usage")
        .description("The account in use now, its 5-hour and weekly limits, and the best one to switch to.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

@main
struct SCASWidgetBundle: WidgetBundle {
    var body: some Widget {
        SCASWidget()
    }
}
