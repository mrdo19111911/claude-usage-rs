// Preview (cargo test preview -- --ignored): renders the widget for preset models to
// target/widget-preview/<name>_<pinned|hover>_<1x|2x>.bmp, to look at without hovering the tray.

use super::*;

const NOW: i64 = 1_760_000_000_000;
const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;

fn win(u: i64, reset_in: i64) -> Win {
    Win { u, reset: Some(NOW + reset_in) }
}

fn usage(five: i64, five_in: i64, week: i64, week_in: i64) -> Option<Use> {
    Some(Use { five: win(five, five_in), week: win(week, week_in) })
}

fn side(email: &str, u: Option<Use>) -> Side {
    Side { email: Some(email.into()), usage: u }
}

fn other(email: &str, u: Option<Use>, desk: bool) -> Other {
    Other { email: email.into(), usage: u, id: email.into(), desk_uuid: desk.then(|| format!("uuid-{email}")), ..Default::default() }
}

fn base() -> Model {
    Model { now: NOW, checked: Some(NOW - MIN), ..Default::default() }
}

fn crowd() -> Vec<Other> {
    let full = |i: i64| usage(100, i * HOUR, 100, i * DAY);
    vec![
        other("bob@example.com", usage(26, 2 * HOUR, 74, 3 * DAY), true),
        other("dave@example.com", full(1), false),
        other("carol@example.com", full(2), false),
        other("erin@example.com", full(3), false),
        other("frank@example.com", full(4), false),
    ]
}

pub(super) fn presets() -> Vec<(&'static str, Model)> {
    let tr = "alice@example.com";
    let u = usage(16, 2 * HOUR + 45 * MIN, 51, DAY + HOUR);
    let same = Model { cli: side(tr, u.clone()), desk: Some(side(tr, u)), others: crowd(), ..base() };
    let two = Model {
        cli: side("a@x.com", usage(30, 3 * HOUR, 40, 2 * DAY)),
        desk: Some(side("b@x.com", usage(80, HOUR, 90, DAY))),
        others: vec![other("c@x.com", usage(10, 4 * HOUR, 20, 5 * DAY), true), other("d@x.com", usage(100, 2 * HOUR, 60, 3 * DAY), false)],
        ..base()
    };
    let no_desk = Model { desk: None, ..two.clone() };
    let blank = |e: &str| Other { usage: None, ..other(e, None, true) };
    let none = Model {
        cli: side("a@x.com", None),
        desk: Some(side("b@x.com", None)),
        others: vec![blank("c@x.com"), blank("d@x.com")],
        checked: None,
        ..base()
    };
    let long = |p: &str| format!("{p}.very.long.address.that.keeps.going.and.going@some-extremely-long-company-domain.example.com");
    let long_names = Model {
        cli: side(&long("cli"), usage(55, 2 * HOUR, 65, 2 * DAY)),
        desk: Some(side(&long("desk"), usage(20, 4 * HOUR, 30, 4 * DAY))),
        others: vec![other(&long("other1"), usage(5, 3 * HOUR, 10, 6 * DAY), true), other(&long("other2"), usage(100, HOUR, 100, DAY), false)],
        ..base()
    };
    let switching = Model { desk_switching: true, ..two.clone() };
    // bob's row open (docs/specs/gadget-chart.md): a week of checks every 20 min.
    let mut open = same.clone();
    open.others[0].history = (0..7 * 72).map(|i| {
        let t = NOW - DAY + 2 * HOUR - 7 * DAY + i * 20 * MIN;
        let into_five = (t - (NOW + 2 * HOUR - 5 * HOUR)).rem_euclid(5 * HOUR);
        (t, 26 * into_five / (3 * HOUR), 74 * i / (7 * 72))
    }).filter(|p| p.0 <= NOW).collect();
    (open.five_line, open.week_line) = (95, 99);
    vec![("open_row", open), ("same_account", same), ("two_accounts", two), ("no_desktop", no_desk), ("no_figures", none), ("long_names", long_names), ("switching", switching)]
}

// Lays out on a memory DC with a top-down 32-bit DIB; returns (height, BGRA pixels).
unsafe fn render(m: &Model, s: f32, pinned: bool) -> (i32, Vec<u8>) {
    let (w, h) = (width(s), layout(m, s, pinned, ptr::null_mut()));
    let dc = CreateCompatibleDC(ptr::null_mut());
    let mut bi: BITMAPINFO = mem::zeroed();
    bi.bmiHeader = BITMAPINFOHEADER { biSize: mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..mem::zeroed() };
    let mut bits = ptr::null_mut();
    let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
    assert!(!bmp.is_null() && !bits.is_null(), "CreateDIBSection failed");
    let old = SelectObject(dc, bmp);
    let drawn = layout(m, s, pinned, dc);
    GdiFlush();
    assert_eq!(drawn, h, "drawing changed the height");
    let px = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize).to_vec();
    SelectObject(dc, old);
    DeleteObject(bmp);
    DeleteDC(dc);
    (h, px)
}

// A top-down 32-bit .bmp: file header + info header + pixels.
fn bmp(w: i32, h: i32, px: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(54 + px.len());
    b.extend(b"BM");
    b.extend((54 + px.len() as u32).to_le_bytes());
    b.extend([0u8; 4]);
    b.extend(54u32.to_le_bytes());
    b.extend(40u32.to_le_bytes());
    b.extend(w.to_le_bytes());
    b.extend((-h).to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(32u16.to_le_bytes());
    b.extend([0u8; 24]);
    b.extend(px);
    b
}

#[test]
fn every_preset_lays_out() {
    for (name, m) in presets() {
        set_open(name.starts_with("open").then(|| "bob@example.com".into()));
        for pinned in [false, true] {
            let h = unsafe { layout(&m, 1.0, pinned, ptr::null_mut()) };
            assert!(h > 0, "{name}: height {h}");
            let (drawn, px) = unsafe { render(&m, 1.0, pinned) };
            assert_eq!(drawn, h, "{name}: height differs on a real DC");
            assert!(px.iter().any(|&p| p != 0), "{name}: nothing drawn");
        }
    }
}

#[test]
#[ignore]
fn preview() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("widget-preview");
    fs::create_dir_all(&dir).unwrap();
    for (name, m) in presets() {
        set_open(name.starts_with("open").then(|| "bob@example.com".into()));
        for (pinned, mode) in [(false, "hover"), (true, "pinned")] {
            for (s, x) in [(1.0, "1x"), (2.0, "2x")] {
                let (h, px) = unsafe { render(&m, s, pinned) };
                let path = dir.join(format!("{name}_{mode}_{x}.bmp"));
                fs::write(&path, bmp(width(s), h, &px)).unwrap();
                println!("{}", path.display());
            }
        }
    }
}
