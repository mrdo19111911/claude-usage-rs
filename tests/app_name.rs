// The app is "Smartlog Claude Auto Switch" everywhere it shows (docs/specs/rename.md): no old name
// left in the code, web pages, build files, installer or macOS workflow, except the names
// kept on purpose and the 1.5.x leftovers Setup removes.

use std::fs;
use std::path::{Path, PathBuf};

// 1.5.x's name, and "Claude AutoSwitch", a name used for an hour on 2026-11-12, never released.
const OLD: [&str; 6] = ["Claude Usage", "claude-usage", "ClaudeUsage", "Claude AutoSwitch", "claude-autoswitch", "ClaudeAutoSwitch"];
// Kept: the data folder, the mutex, the repo, a sibling project named in a comment.
const KEPT: [&str; 3] = ["claude-usage-dashboard", "claude-usage-rs", "claude-usage-monitor"];
// Leftovers of 1.5.x, named only to be removed by Setup.
const LEFTOVERS: [&str; 3] = [r"{autopf}\Claude Usage", r"{autoprograms}\Claude Usage.lnk", "ValueName: \"ClaudeUsage\"; ValueType: none"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("rs" | "html" | "js" | "css")) {
            out.push(p);
        }
    }
}

fn files() -> Vec<PathBuf> {
    let r = root();
    let mut out = vec![];
    walk(&r.join("src"), &mut out);
    for f in ["build.rs", "Cargo.toml", "installer/scas.iss", "installer/build.ps1", ".github/workflows/macos.yml"] {
        out.push(r.join(f));
    }
    out
}

// What is left of a line once the allowed names are cut out.
fn stale(line: &str) -> Option<&'static str> {
    let mut l = line.to_string();
    for k in LEFTOVERS.iter().chain(KEPT.iter()) {
        l = l.replace(k, "");
    }
    OLD.into_iter().find(|o| l.contains(o))
}

#[test]
fn no_old_name_is_left() {
    let mut found = vec![];
    for f in files() {
        let text = fs::read_to_string(&f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        for (i, l) in text.lines().enumerate() {
            if let Some(o) = stale(l) {
                found.push(format!("{}:{}: {o}: {}", f.strip_prefix(root()).unwrap().display(), i + 1, l.trim()));
            }
        }
    }
    assert!(found.is_empty(), "old name left:\n{}", found.join("\n"));
}

#[test]
fn the_check_catches_old_names_and_spares_kept_ones() {
    assert_eq!(stale("<title>Claude Usage</title>"), Some("Claude Usage"));
    assert_eq!(stale("ClaudeUsage-Setup-1.0.exe"), Some("ClaudeUsage"));
    assert_eq!(stale(r#"home().join(".claude-usage-dashboard")"#), None);
    assert_eq!(stale("https://github.com/mrdo19111911/claude-usage-rs"), None);
}

#[test]
fn the_package_and_its_program_are_scas() {
    let cargo = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(cargo.lines().any(|l| l.trim() == r#"name = "scas""#), "Cargo.toml package name");
    let publish = fs::read_to_string(root().join("tools/publish.py")).unwrap();
    assert!(publish.contains(r"scas\.iss"), "publish.py keeps the new installer script");
}
