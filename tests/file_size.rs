// Every source file stays at 200 lines or fewer (docs/specs/modules.md), so each one
// can be read, reviewed and changed on its own.

use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINES: usize = 200;

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn no_source_file_is_over_200_lines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut all = vec![];
    for dir in ["src", "tests"] {
        files(&root.join(dir), &mut all);
    }
    let mut over: Vec<String> = all
        .iter()
        .filter_map(|p| {
            let n = fs::read_to_string(p).ok()?.lines().count();
            (n > MAX_LINES).then(|| format!("{} ({n} lines)", p.strip_prefix(root).unwrap().display()))
        })
        .collect();
    over.sort();
    assert!(over.is_empty(), "files over {MAX_LINES} lines:\n  {}", over.join("\n  "));
}
