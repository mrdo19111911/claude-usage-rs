// Tests of where the dashboard window opens (docs/specs/dashboard-window.md).

use super::*;
use serde_json::json;

const WORK: Rect = Rect { x: 0, y: 0, w: 1920, h: 1040 };

#[test]
fn no_file_opens_the_default_size_centred() {
    assert_eq!(first_rect(None, WORK, 1.0), Rect { x: 370, y: 110, w: 1180, h: 820 });
}

#[test]
fn the_default_size_follows_the_dpi() {
    let work = Rect { x: 0, y: 0, w: 2560, h: 1400 };
    assert_eq!(first_rect(None, work, 1.5), Rect { x: 395, y: 85, w: 1770, h: 1230 });
}

#[test]
fn never_bigger_than_the_work_area() {
    let small = Rect { x: 100, y: 50, w: 1000, h: 700 };
    assert_eq!(first_rect(None, small, 1.0), small);
}

#[test]
fn a_saved_place_on_screen_is_kept() {
    let saved = Rect { x: 200, y: 100, w: 900, h: 600 };
    assert_eq!(first_rect(Some(saved), WORK, 1.0), saved);
    // Its size is still capped by the screen.
    let big = Rect { x: 0, y: 0, w: 4000, h: 3000 };
    assert_eq!(first_rect(Some(big), WORK, 1.0), Rect { x: 0, y: 0, w: 1920, h: 1040 });
}

#[test]
fn a_saved_place_off_screen_gives_the_default() {
    // On a screen unplugged since: the title bar would be out of reach.
    let gone = Rect { x: 2500, y: 100, w: 900, h: 600 };
    assert_eq!(first_rect(Some(gone), WORK, 1.0), first_rect(None, WORK, 1.0));
    let above = Rect { x: 200, y: -700, w: 900, h: 600 };
    assert_eq!(first_rect(Some(above), WORK, 1.0), first_rect(None, WORK, 1.0));
}

#[test]
fn the_place_goes_through_its_file() {
    let r = Rect { x: -5, y: 10, w: 900, h: 600 };
    assert_eq!(rect_from_json(&rect_json(r)), Some(r));
    assert_eq!(rect_from_json(&json!({})), None);
    assert_eq!(rect_from_json(&json!({"x": 1, "y": 2, "w": 0, "h": 5})), None);
}
