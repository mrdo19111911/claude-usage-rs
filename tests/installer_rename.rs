// Setup under the new name (docs/specs/rename.md): new names shown and installed, the same
// AppId and mutex as 1.5.x so it upgrades in place, and 1.5.x's leftovers removed.

use std::fs;
use std::path::Path;

fn iss() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("installer/scas.iss")).unwrap()
}

fn has(text: &str, line: &str) -> bool {
    text.lines().any(|l| l.trim() == line)
}

#[test]
fn shows_and_installs_the_new_name() {
    let t = iss();
    for l in [
        r#"#define AppName "Smartlog Claude Auto Switch""#,
        r#"#define AppExe "scas.exe""#,
        // The install folder: the short name.
        r#"#define AppDir "SCAS""#,
        "OutputBaseFilename=SCAS-Setup-{#AppVersion}",
    ] {
        assert!(has(&t, l), "missing: {l}");
    }
    assert!(t.contains(r#"ValueName: "SCAS"; ValueData: """{app}\{#AppExe}""""#), "Run value");
}

#[test]
fn upgrades_the_old_install_in_place() {
    let t = iss();
    assert!(has(&t, "AppId={{63905BC9-FAEF-420D-8FC1-DF04B915B4AF}"), "AppId must never change");
    assert!(t.contains(r"Mutex = 'Local\claude-usage-dashboard';"), "the mutex 1.5.x holds");
    assert!(has(&t, "UsePreviousAppDir=no"), "installs into the new folder");
}

#[test]
fn removes_what_1_5_left() {
    let t = iss();
    assert!(has(&t, r#"Type: filesandordirs; Name: "{autopf}\Claude Usage""#), "old folder");
    assert!(has(&t, r#"Type: files; Name: "{autoprograms}\Claude Usage.lnk""#), "old shortcut");
    let run = r#"Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "ClaudeUsage"; ValueType: none; Flags: deletevalue"#;
    assert!(has(&t, run), "old Run value");
}
