// Tests of model.rs's times: how long is left, the offset from UTC.

use super::*;

#[test]
fn time_left() {
    assert_eq!(left(4 * 3_600_000 + 39 * 60_000), "4h39m");
    assert_eq!(left(51 * 3_600_000), "2d 3h");
    assert_eq!(left(30_000), "1m");
}

// The chart's lines use the computer's offset from UTC: it agrees with clock().
#[test]
fn utc_offset_agrees_with_the_clock() {
    let t = 1_800_000_000_000i64;
    let off = utc_offset(t);
    assert!(off.abs() <= 14 * 3_600_000 && off % (15 * 60_000) == 0, "{off}");
    let local = (t + off).rem_euclid(86_400_000) / 60_000;
    assert_eq!(clock(t, t), format!("{:02}:{:02}", local / 60, local % 60));
}
