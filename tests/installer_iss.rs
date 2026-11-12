// The installer's install mode (docs/specs/installer.md): for all users by default (admin,
// shortcut in the common Start menu), "only for me" on the same dialog, and nothing that
// would put per-user paths or an elevated app into an all-users install.

use std::fs;
use std::path::Path;

fn iss() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("installer/claude-usage.iss");
    fs::read_to_string(p).unwrap()
}

// Lines of one [Section], comments and blanks dropped.
fn section(text: &str, name: &str) -> Vec<String> {
    let head = format!("[{name}]");
    let mut inside = false;
    let mut out = vec![];
    for l in text.lines().map(str::trim) {
        if l.starts_with('[') && l.ends_with(']') {
            inside = l == head;
        } else if inside && !l.is_empty() && !l.starts_with(';') {
            out.push(l.to_string());
        }
    }
    out
}

#[test]
fn installs_for_all_users_by_default_and_lets_the_user_choose() {
    let setup = section(&iss(), "Setup");
    assert!(setup.contains(&"PrivilegesRequired=admin".to_string()), "{setup:?}");
    assert!(setup.contains(&"PrivilegesRequiredOverridesAllowed=dialog".to_string()), "{setup:?}");
}

#[test]
fn folders_and_registry_follow_the_chosen_mode() {
    let text = iss();
    let setup = section(&text, "Setup");
    assert!(setup.contains(&"DefaultDirName={autopf}\\{#AppName}".to_string()), "{setup:?}");
    for l in section(&text, "Icons") {
        assert!(l.starts_with("Name: \"{autoprograms}\\"), "{l}");
    }
    for l in section(&text, "Registry") {
        assert!(l.starts_with("Root: HKA;"), "{l}");
    }
    for fixed in ["{localappdata}", "{userprograms}", "{commonprograms}", "Root: HKCU", "Root: HKLM"] {
        assert!(!text.contains(fixed), "fixed per-user or per-machine path left: {fixed}");
    }
}

#[test]
fn the_app_never_starts_elevated() {
    let run = section(&iss(), "Run");
    assert!(!run.is_empty());
    for l in run {
        assert!(l.contains("runasoriginaluser"), "would start as the admin account: {l}");
    }
}
