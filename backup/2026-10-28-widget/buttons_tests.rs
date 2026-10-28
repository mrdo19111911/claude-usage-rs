// The row buttons: which ones a row gets, where, and the hit test.

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

#[test]
fn a_free_account_gets_both_right_aligned() {
    let b = row_buttons(&model_with_desk(), &other(), 360, 1.0, 100);
    assert_eq!(picks(&b), [Pick::Cli("b".into()), Pick::Desktop { uuid: "u-b".into(), email: "b@x.com".into() }]);
    let (c, d) = (b[0].0, b[1].0);
    // Desktop ends at the padding; CLI sits left of it with a gap.
    assert_eq!(d.right, 360 - 14);
    assert!(c.right < d.left);
    assert_eq!((c.top, c.bottom, d.top, d.bottom), (100, 118, 100, 118));
    assert!(text_right(360, 1.0) < c.left);
    // Room left for the name and the figures at 360 px.
    assert!(text_right(360, 1.0) - 14 >= 230);
}

#[test]
fn no_cli_button_when_it_is_claude_clis_now() {
    let o = Other { in_cli: true, ..other() };
    assert_eq!(picks(&row_buttons(&model_with_desk(), &o, 360, 1.0, 0)), [Pick::Desktop { uuid: "u-b".into(), email: "b@x.com".into() }]);
}

#[test]
fn no_desktop_button_in_four_cases() {
    let cli_only = [Pick::Cli("b".into())];
    let m = model_with_desk();
    assert_eq!(picks(&row_buttons(&m, &Other { in_desk: true, ..other() }, 360, 1.0, 0)), cli_only);
    assert_eq!(picks(&row_buttons(&m, &Other { desk_uuid: None, ..other() }, 360, 1.0, 0)), cli_only);
    assert_eq!(picks(&row_buttons(&Model::default(), &other(), 360, 1.0, 0)), cli_only);
    let busy = Model { desk_switching: true, ..model_with_desk() };
    assert_eq!(picks(&row_buttons(&busy, &other(), 360, 1.0, 0)), cli_only);
}

#[test]
fn a_missing_cli_button_keeps_desktop_in_its_slot() {
    let both = row_buttons(&model_with_desk(), &other(), 360, 1.0, 0);
    let one = row_buttons(&model_with_desk(), &Other { in_cli: true, ..other() }, 360, 1.0, 0);
    assert_eq!((one[0].0.left, one[0].0.right), (both[1].0.left, both[1].0.right));
}

#[test]
fn buttons_grow_with_the_scale() {
    let b = row_buttons(&model_with_desk(), &other(), 720, 2.0, 50);
    let d = b[1].0;
    assert_eq!((d.right, d.right - d.left, d.bottom - d.top), (720 - 28, 108, 36));
}

#[test]
fn hit_finds_the_button_under_the_point() {
    let b = row_buttons(&model_with_desk(), &other(), 360, 1.0, 100);
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
    let b = row_buttons(&model_with_desk(), &other(), 360, 1.0, 100);
    let c = b[0].0;
    set_hits(true, b);
    set_hits(false, vec![]);
    assert_eq!(hit_at(true, c.left + 1, c.top + 1), Some(Pick::Cli("b".into())));
    assert_eq!(hit_at(false, c.left + 1, c.top + 1), None);
}

#[test]
fn a_pick_becomes_its_act() {
    assert!(matches!(Pick::Cli("b".into()).act(), Act::Cli(id) if id == "b"));
    assert!(matches!(Pick::Desktop { uuid: "u".into(), email: "e".into() }.act(), Act::Desktop(u) if u == "u"));
}
