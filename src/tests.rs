// Tests of the core (main.rs before the split).

#[test]
fn thins_old_history_only() {
    let (h, t) = (3_600_000, 10 * 24 * 3_600_000);
    let mut pts = vec![(t - 8 * 24 * h, 1, 1)]; // past 7 days: dropped
    for i in 0..30 {
        pts.push((t - 10 * h + i * 60_000, i, i)); // 30 points a minute apart, 10 h ago
    }
    for i in 0..30 {
        pts.push((t - h + i * 60_000, i, i)); // 30 points in the last hour: all kept
    }
    let out = super::thin_history(&pts, t);
    let old: Vec<_> = out.iter().filter(|p| p.0 < t - 6 * h).collect();
    assert!(old.len() <= 4 && !old.is_empty(), "one per 10 min: {}", old.len());
    assert_eq!(old.last().unwrap().1, 29, "keeps the last of each bucket");
    assert_eq!(out.iter().filter(|p| p.0 >= t - 6 * h).count(), 30);
}

#[test]
fn checks_less_when_far_from_a_limit_and_not_at_all_while_one_is_full() {
    use super::{next_check_at, Usage, Win};
    let w = |u, r| Win { u, resets_at: r };
    let at = |five, week| Usage { five, week, fetched_at: 0, scoped: vec![] };
    let (base, t, h) = (120_000, 1_000_000, 3_600_000);
    // New account, little used: every base.
    assert_eq!(next_check_at(&at(w(5, None), w(10, None)), base, t), t + 120_000);
    // Nearer a limit (either window): more often.
    assert_eq!(next_check_at(&at(w(72, None), w(10, None)), base, t), t + 60_000);
    assert_eq!(next_check_at(&at(w(10, None), w(90, None)), base, t), t + 30_000);
    assert_eq!(next_check_at(&at(w(96, None), w(10, None)), base, t), t + 15_000);
    assert_eq!(next_check_at(&at(w(99, None), w(10, None)), 60_000, t), t + 15_000, "never under 15 s");
    // Weekly full, resets in 3 h: nothing until then (the 5-hour figure cannot help).
    assert_eq!(next_check_at(&at(w(0, Some(t + 2 * h)), w(100, Some(t + 3 * h))), base, t), t + 3 * h + 60_000);
    // 5-hour full: until it resets.
    assert_eq!(next_check_at(&at(w(100, Some(t + h)), w(40, Some(t + 50 * h))), base, t), t + h + 60_000);
    // Full but its reset time unknown or past: ask again soon, as near a limit.
    assert_eq!(next_check_at(&at(w(100, None), w(40, None)), base, t), t + 15_000);
    assert_eq!(next_check_at(&at(w(100, Some(t - 1)), w(40, None)), base, t), t + 15_000);
}

#[test]
fn notes_desktop_session_names() {
    let d = std::env::temp_dir().join(format!("cud-names-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let u = "fb2bda4d-0000-4000-8000-000000000001";
    assert_eq!(super::noted_email(&d, u), None);
    assert!(super::note_session(&d, u, "a@example.com"));
    assert_eq!(super::noted_email(&d, u).as_deref(), Some("a@example.com"));
    assert!(super::note_session(&d, u, "b@example.com"), "a new email replaces the old");
    assert_eq!(super::noted_email(&d, u).as_deref(), Some("b@example.com"));
    assert!(!super::note_session(&d, "../../evil", "a@example.com"), "uuid is a file name: checked");
    assert!(!super::note_session(&d, u, "not an email"));
    assert_eq!(super::noted_email(&d, "../x"), None);
}

#[test]
fn backs_off_after_429() {
    use super::{backoff_ms, retry_after_ms};
    assert_eq!(retry_after_ms(Some("0")), Some(0));
    assert_eq!(retry_after_ms(Some("120")), Some(120_000));
    assert_eq!(retry_after_ms(Some("Wed, 21 Oct 2026 07:28:00 GMT")), None);
    // Retry-After 0 (what Anthropic sent on 2026-10-28) names no wait: 5 min, doubling.
    let mins: Vec<i64> = (1..=6).map(|n| backoff_ms(Some(0), n) / 60_000).collect();
    assert_eq!(mins, vec![5, 10, 20, 40, 40, 40]);
    assert_eq!(backoff_ms(Some(90_000), 3), 90_000, "a named wait is kept");
    assert_eq!(backoff_ms(Some(5 * 3_600_000), 1), 3_600_000, "at most an hour");
}

#[test]
fn iso() {
    assert_eq!(super::parse_iso("2025-10-28T11:40:00.123456+00:00"), Some(1761651600123));
    assert_eq!(super::parse_iso("2025-10-28T18:40:00+07:00"), Some(1761651600000));
    assert_eq!(super::parse_iso("2024-02-29T00:00:00Z"), Some(1709164800000));
}

#[test]
fn per_model_weekly_limits_are_read() {
    use crate::*;
    let j = json!({"limits": [
        {"kind": "session", "percent": 0, "resets_at": null, "scope": null},
        {"kind": "weekly_all", "percent": 53, "resets_at": "2026-11-02T01:59:59.5+00:00", "scope": null},
        {"kind": "weekly_scoped", "percent": 7, "resets_at": "2026-11-02T02:00:00+00:00",
         "scope": {"model": {"id": null, "display_name": "Fable"}, "surface": null}}
    ]});
    let s = to_scoped(&j);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].name.as_str(), s[0].u), ("Fable", 7));
    assert_eq!(s[0].resets_at, parse_iso("2026-11-02T02:00:00+00:00"));
    assert!(to_scoped(&json!({"five_hour": {}})).is_empty(), "no limits list: none");
}

#[test]
fn auto_switch_has_a_threshold_per_window() {
    use crate::switch::{has_room, switch_reason};
    use crate::*;
    let s: Settings = serde_json::from_str(r#"{"threshold":95,"autoSwitch":true,"pollSeconds":120}"#).unwrap();
    assert_eq!((s.threshold, s.week_threshold), (95, 99), "old settings: weekly 99");
    let at = |five, week| Usage { five: Win { u: five, resets_at: None }, week: Win { u: week, resets_at: None }, fetched_at: 0, scoped: vec![] };
    assert_eq!(switch_reason(&at(10, 96), &s), None, "weekly 96% is not the end yet");
    assert_eq!(switch_reason(&at(10, 99), &s).as_deref(), Some("weekly 99%"));
    assert_eq!(switch_reason(&at(95, 40), &s).as_deref(), Some("5-hour 95%"));
    assert!(has_room(&at(20, 97), &s), "weekly 97% can still take over");
    assert!(!has_room(&at(95, 10), &s) && !has_room(&at(10, 99), &s));
}
