// docs/specs/no-foreign-credentials.md: SCAS never reads Claude Desktop's tokens and
// never kills Desktop (a killed Desktop can leave a stale cookie that a switch restores).

use std::fs;
use std::path::Path;

const BANNED: [&str; 5] = ["CryptUnprotectData", "decrypt_v10", "safe_storage", "TerminateProcess", "desktop_access_token"];

fn scan(dir: &Path, hits: &mut Vec<String>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            scan(&p, hits);
        } else if p.extension().is_some_and(|x| x == "rs") {
            let text = fs::read_to_string(&p).unwrap();
            for b in BANNED.iter().filter(|b| text.contains(*b)) {
                hits.push(format!("{}: {b}", p.display()));
            }
        }
    }
}

#[test]
fn no_desktop_token_reading_and_no_killing_desktop() {
    let mut hits = vec![];
    scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut hits);
    assert!(hits.is_empty(), "banned by docs/specs/no-foreign-credentials.md:\n{}", hits.join("\n"));
}
