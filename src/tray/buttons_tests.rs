// The row buttons: which ones a row gets, where, the hit test, hover and the footer.

use super::*;

fn other() -> Other {
    Other { email: "b@x.com".into(), id: "b".into(), desk_uuid: Some("u-b".into()), ..Default::default() }
}

fn model_with_desk() -> Model {
    Model { desk: Some(Side::default()), ..Default::default() }
}

fn picks(b: &[(RECT, Pick)]) -> Vec<Pick> {
    b.iter().map(|(_, k)| k.clone()).collect()
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT { left, top, right, bottom }
}

#[test]
fn a_free_account_gets_both_right_aligned() {
    let b = row_buttons(&model_with_desk(), &other(), 300, 1.0, 100);
    assert_eq!(picks(&b), [Pick::Cli("b".into()), Pick::Desktop { uuid: "u-b".into(), email: "b@x.com".into() }]);
    let (c, d) = (b[0].0, b[1].0);
    // Desktop ends at the padding; CLI sits left of it with a gap.
    assert_eq!((d.left, d.right), (300 - 14 - 58, 300 - 14));
    assert_eq!((c.left, c.right), (d.left - 4 - 40, d.left - 4));
    assert_eq!((c.top, c.bottom, d.top, d.bottom), (100, 118, 100, 118));
    assert_eq!(text_right(300, 1.0), c.left - 8);
    // Room left for the name and the figures at 300 px.
    assert!(text_right(300, 1.0) - 14 >= 140);
}

#[test]
fn no_cli_button_when_it_is_claude_clis_now() {
    let o = Other { in_cli: true, ..other() };
    assert_eq!(picks(&row_buttons(&model_with_desk(), &o, 300, 1.0, 0)), [Pick::Desktop { uuid: "u-b".into(), email: "b@x.com".into() }]);
}

#[test]
fn no_desktop_button_in_four_cases() {
    let cli_only = [Pick::Cli("b".into())];
    let m = model_with_desk();
    assert_eq!(picks(&row_buttons(&m, &Other { in_desk: true, ..other() }, 300, 1.0, 0)), cli_only);
    assert_eq!(picks(&row_buttons(&m, &Other { desk_uuid: None, ..other() }, 300, 1.0, 0)), cli_only);
    assert_eq!(picks(&row_buttons(&Model::default(), &other(), 300, 1.0, 0)), cli_only);
    let busy = Model { desk_switching: true, ..model_with_desk() };
    assert_eq!(picks(&row_buttons(&busy, &other(), 300, 1.0, 0)), cli_only);
}

#[test]
fn a_missing_button_leaves_its_slot_empty() {
    let both = row_buttons(&model_with_desk(), &other(), 300, 1.0, 0);
    let no_cli = row_buttons(&model_with_desk(), &Other { in_cli: true, ..other() }, 300, 1.0, 0);
    assert_eq!((no_cli[0].0.left, no_cli[0].0.right), (both[1].0.left, both[1].0.right));
    let no_desk = row_buttons(&Model::default(), &other(), 300, 1.0, 0);
    assert_eq!((no_desk[0].0.left, no_desk[0].0.right), (both[0].0.left, both[0].0.right));
}

#[test]
fn buttons_grow_with_the_scale() {
    let b = row_buttons(&model_with_desk(), &other(), 600, 2.0, 50);
    let (c, d) = (b[0].0, b[1].0);
    assert_eq!((d.right, d.right - d.left, d.top, d.bottom - d.top), (600 - 28, 116, 50, 36));
    assert_eq!((c.right, c.right - c.left), (d.left - 8, 80));
    assert_eq!(text_right(600, 2.0), c.left - 16);
}

#[test]
fn hit_finds_the_button_under_the_point() {
    let b = row_buttons(&model_with_desk(), &other(), 300, 1.0, 100);
    let (c, d) = (b[0].0, b[1].0);
    assert_eq!(hit(&b, c.left, c.top), Some(Pick::Cli("b".into())));
    assert_eq!(hit(&b, d.right - 1, d.bottom - 1), Some(Pick::Desktop { uuid: "u-b".into(), email: "b@x.com".into() }));
    // Edges right and bottom are outside; so are the gap and the row's text.
    assert_eq!(hit(&b, d.right, d.top), None);
    assert_eq!(hit(&b, c.left, c.bottom), None);
    assert_eq!(hit(&b, c.right, c.top + 1), None);
    assert_eq!(hit(&b, 20, 105), None);
    assert_eq!(hit(&[], 20, 105), None);
}

#[test]
fn hits_are_kept_per_window() {
    let b = row_buttons(&model_with_desk(), &other(), 300, 1.0, 100);
    let c = b[0].0;
    set_hits(true, b);
    set_hits(false, vec![]);
    assert_eq!(hit_at(true, c.left + 1, c.top + 1), Some(Pick::Cli("b".into())));
    assert_eq!(hit_at(false, c.left + 1, c.top + 1), None);
}

#[test]
fn the_footer_is_kept_per_window() {
    set_footer(true, rect(0, 200, 300, 230));
    set_footer(false, rect(0, 300, 300, 330));
    assert!(in_footer(true, 10, 200));
    assert!(!in_footer(true, 10, 230));
    assert!(!in_footer(false, 10, 210));
    assert!(in_footer(false, 299, 329));
}

#[test]
fn hot_follows_the_pointer_of_that_window() {
    let r = rect(10, 10, 50, 28);
    set_pointer(true, Some((20, 15)));
    set_pointer(false, None);
    assert!(hot(true, &r));
    assert!(!hot(false, &r));
    set_pointer(true, Some((50, 15)));
    assert!(!hot(true, &r));
    set_pointer(true, None);
    assert!(!hot(true, &r));
}

#[test]
fn spot_names_what_the_pointer_is_over() {
    let head = rect(266, 8, 290, 32);
    let b = row_buttons(&model_with_desk(), &other(), 300, 1.0, 100);
    let d = b[1].0;
    set_hits(false, b);
    set_footer(false, rect(0, 200, 300, 230));
    assert_eq!(spot(false, 270, 10, &head), Spot::Head);
    assert_eq!(spot(false, 5, 205, &head), Spot::Footer);
    assert_eq!(spot(false, d.left, d.top, &head), Spot::Button(1));
    assert_eq!(spot(false, 5, 105, &head), Spot::None);
    // Nothing kept for the other window.
    set_hits(true, vec![]);
    set_footer(true, rect(0, 0, 0, 0));
    assert_eq!(spot(true, d.left, d.top, &head), Spot::None);
}

#[test]
fn begin_draw_names_the_window_being_drawn() {
    begin_draw(true);
    assert!(drawing());
    begin_draw(false);
    assert!(!drawing());
}

// A button outlined at rest, filled when the pointer is over it.
#[test]
fn a_hot_button_is_filled() {
    unsafe {
        let screen = GetDC(ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, 300, 40);
        let old = SelectObject(dc, bmp);
        let f = CreateFontW(-12, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 0, 0, wide("Segoe UI").as_ptr());
        let p = Painter { dc, s: 1.0, fonts: [f; 4] };
        let b = row_buttons(&model_with_desk(), &other(), 300, 1.0, 10);
        let (c, d) = (b[0].0, b[1].0);
        p.fill(0, 0, 300, 40, BG);
        begin_draw(false);
        set_pointer(false, Some((c.left + 3, c.top + 3)));
        draw_buttons(&p, &b);
        // Inside, away from the label: the hot CLI button is filled, Desktop is not.
        assert_eq!(GetPixel(dc, c.left + 3, c.bottom - 4), CODE);
        assert_eq!(GetPixel(dc, d.left + 3, d.bottom - 4), BG);
        // Desktop's outline is in its colour.
        assert_eq!(GetPixel(dc, d.left, (d.top + d.bottom) / 2), DESK);
        set_pointer(false, None);
        SelectObject(dc, old);
        DeleteObject(bmp);
        DeleteDC(dc);
        DeleteObject(f);
        ReleaseDC(ptr::null_mut(), screen);
    }
}

#[test]
fn a_pick_becomes_its_act() {
    assert!(matches!(Pick::Cli("b".into()).act(), Act::Cli(id) if id == "b"));
    assert!(matches!(Pick::Desktop { uuid: "u".into(), email: "e".into() }.act(), Act::Desktop(u) if u == "u"));
}
