// Tests for chart_data.rs: the points of a row's chart (docs/specs/gadget-chart.md).

use super::*;

const H: i64 = 3_600_000;

#[test]
fn a_reset_that_has_passed_counts_as_zero_and_is_remembered() {
    assert_eq!(eff(&Win { u: 80, reset: Some(5 * H) }, 6 * H), (0, None, Some(5 * H)));
    assert_eq!(eff(&Win { u: 80, reset: Some(9 * H) }, 6 * H), (80, Some(9 * H), None));
    assert_eq!(eff(&Win { u: 7, reset: None }, 6 * H), (7, None, None));
}

#[test]
fn the_span_ends_at_the_reset_or_now() {
    assert_eq!(span(Some(10 * H), 8 * H, 5 * H), (5 * H, 10 * H));
    assert_eq!(span(None, 8 * H, 5 * H), (3 * H, 8 * H));
    assert_eq!(span(Some(7 * H), 8 * H, 5 * H), (3 * H, 8 * H), "a past reset: up to now");
}

#[test]
fn points_start_at_zero_and_carry_the_figure_to_now() {
    let hist = [(H, 50, 1), (6 * H, 10, 20), (7 * H, 30, 22)];
    // 5-hour, span 5h..10h: the older figure is before the window, so it starts at 0.
    let p = points(&hist, false, 5 * H, 8 * H, Some(35), None);
    assert_eq!(p, vec![(5 * H, 0), (6 * H, 10), (7 * H, 30), (8 * H, 35)]);
    // Weekly column, nothing before the window: no start point.
    let p = points(&hist[1..], true, 5 * H, 8 * H, Some(23), None);
    assert_eq!(p, vec![(6 * H, 20), (7 * H, 22), (8 * H, 23)]);
}

#[test]
fn points_drop_to_zero_at_a_passed_reset() {
    let hist = [(4 * H, 60, 0), (5 * H, 90, 0), (7 * H, 99, 0)];
    let p = points(&hist, false, 3 * H, 8 * H, Some(0), Some(6 * H));
    assert_eq!(p, vec![(4 * H, 60), (5 * H, 90), (6 * H, 90), (6 * H, 0), (8 * H, 0)]);
}

#[test]
fn no_history_and_no_figure_gives_no_points() {
    assert!(points(&[], false, 0, 5 * H, None, None).is_empty());
}

#[test]
fn xy_stays_inside_the_box() {
    let p = [(0, 0), (50, 100), (100, 50)];
    let xy = to_xy(&p, 0, 100, (10.0, 20.0, 110.0, 70.0));
    assert_eq!(xy, vec![(10.0, 70.0), (60.0, 20.0), (110.0, 45.0)]);
    for (x, y) in to_xy(&[(-5, 140), (200, -3)], 0, 100, (10.0, 20.0, 110.0, 70.0)) {
        assert!((10.0..=110.0).contains(&x) && (20.0..=70.0).contains(&y));
    }
}

#[test]
fn thin_keeps_first_min_max_last_per_pixel_column() {
    // 1000 points over 300 columns: at most 4 per column, peaks kept.
    let pts: Vec<(f32, f32)> = (0..1000).map(|i| (i as f32 * 0.3, if i == 501 { 3.0 } else { 50.0 + (i % 7) as f32 })).collect();
    let t = thin(&pts);
    assert!(t.len() <= 4 * 300, "{} points", t.len());
    assert!(t.iter().any(|p| p.1 == 3.0), "the spike is kept");
    assert!(t.iter().any(|p| p.1 == 56.0), "the column max is kept");
    assert_eq!(t.first(), pts.first());
    assert_eq!(t.last(), pts.last());
    assert!(t.windows(2).all(|w| w[0].0 <= w[1].0), "time order");
    // Few points: unchanged.
    let few = [(0.0, 1.0), (5.0, 2.0), (9.0, 3.0)];
    assert_eq!(thin(&few), few.to_vec());
}

// The vertical lines, as on the dashboard's charts: each whole hour of a 5-hour window,
// each local midnight of a week. `off`: the computer's offset from UTC.
#[test]
fn lines_at_each_hour_or_each_local_midnight() {
    let d = 24 * H;
    // 5-hour window 10:30 to 15:30 UTC: 11, 12, 13, 14, 15.
    let s = 100 * d + 10 * H + H / 2;
    assert_eq!(ticks(s, s + 5 * H, false, 0), (11..=15).map(|h| 100 * d + h * H).collect::<Vec<_>>());
    // At UTC+7, a whole hour is still a whole hour.
    assert_eq!(ticks(s, s + 5 * H, false, 7 * H).len(), 5);
    // A week from day 100 at 05:00 UTC; at UTC+7 local midnight is 17:00 UTC the day before.
    let s = 100 * d + 5 * H;
    let t = ticks(s, s + 7 * d, true, 7 * H);
    assert_eq!(t.len(), 7);
    assert_eq!(t[0], 100 * d + 17 * H, "first local midnight after the start");
    assert!(t.windows(2).all(|w| w[1] - w[0] == d));
    // A start right on a whole hour does not get a line at the edge.
    assert_eq!(ticks(100 * d, 100 * d + 5 * H, false, 0)[0], 100 * d + H);
}
