// Where the hover widget goes, for a taskbar on each side.

use super::*;

const W: i32 = 360;
const H: i32 = 200;
const GAP: i32 = 12;

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT { left, top, right, bottom }
}

#[test]
fn bottom_taskbar_puts_widget_above_icon() {
    let wa = rect(0, 0, 1920, 1040);
    let (x, y) = hover_pos(1000, 1060, W, H, wa, GAP);
    assert_eq!((x, y), (820, 828));
    assert!(y + H <= wa.bottom && x >= wa.left && x + W <= wa.right);
}

#[test]
fn icon_near_right_edge_is_clamped() {
    let wa = rect(0, 0, 1920, 1040);
    let (x, _) = hover_pos(1900, 1060, W, H, wa, GAP);
    assert_eq!(x, 1920 - W - GAP);
}

#[test]
fn icon_near_left_edge_is_clamped() {
    let wa = rect(0, 0, 1920, 1040);
    let (x, _) = hover_pos(10, 1060, W, H, wa, GAP);
    assert_eq!(x, GAP);
}

#[test]
fn top_taskbar_puts_widget_below_top() {
    let wa = rect(0, 40, 1920, 1080);
    let (_, y) = hover_pos(1000, 20, W, H, wa, GAP);
    assert_eq!(y, 40 + GAP);
}

#[test]
fn side_taskbar_puts_widget_beside_and_above_icon() {
    let wa = rect(60, 0, 1920, 1080);
    let (x, y) = hover_pos(30, 900, W, H, wa, GAP);
    assert_eq!((x, y), (60 + GAP, 900 - H - GAP));
}

#[test]
fn side_taskbar_icon_near_top_stays_inside() {
    let wa = rect(60, 0, 1920, 1080);
    let (_, y) = hover_pos(30, 100, W, H, wa, GAP);
    assert_eq!(y, GAP);
}

#[test]
fn screen_narrower_than_widget_keeps_left_edge() {
    let wa = rect(0, 0, 300, 1040);
    let (x, _) = hover_pos(150, 1060, W, H, wa, GAP);
    assert_eq!(x, 0);
}
