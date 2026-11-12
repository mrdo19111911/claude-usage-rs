// Where updates cannot be installed (macOS: the release's setup is a Windows .exe), the
// check still tells, and nothing is downloaded (docs/specs/macos.md).

use super::*;

fn app() -> App {
    let dir = std::env::temp_dir().join(format!("cud-plat-{}", std::process::id()));
    crate::usage_log::tests::test_app(&dir)
}

#[test]
fn where_updates_do_not_install_the_install_is_refused_before_any_download() {
    let a = app();
    a.update.lock().unwrap().phase = Phase::Available;
    let e = a.begin_install_on(false).unwrap_err();
    assert!(e.contains("Windows only") && e.contains("release page"), "{e}");
    assert_eq!(a.update.lock().unwrap().phase, Phase::Available, "nothing started: no download");
}

#[test]
fn where_updates_install_the_install_starts() {
    let a = app();
    a.update.lock().unwrap().phase = Phase::Available;
    a.begin_install_on(true).unwrap();
    assert_eq!(a.update.lock().unwrap().phase, Phase::Downloading);
}

#[test]
fn a_found_version_waits_to_install_only_where_it_can_install() {
    assert!(waits_to_install(true, true, true));
    assert!(!waits_to_install(true, true, false), "macOS: starts the 3 days like autoUpdate off");
    assert!(!waits_to_install(true, false, true));
    assert!(!waits_to_install(false, true, true));
}

#[test]
fn this_build_installs_updates_only_on_windows() {
    assert_eq!(INSTALLS, cfg!(windows));
}

#[test]
fn the_page_is_told_what_this_build_can_do() {
    let a = app();
    let v: Value = serde_json::from_str(&a.public_state()).unwrap();
    assert_eq!(v["canInstallUpdates"], json!(cfg!(windows)));
    assert_eq!(v["desktop"]["supported"], json!(cfg!(windows)));
}
