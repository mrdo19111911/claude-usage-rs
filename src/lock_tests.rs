use super::*;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cud-lock-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn excludes_a_second_holder_and_releases_on_drop() {
    let d = dir("excl");
    let a = storage(&d).expect("first");
    assert!(d.join(".storage-write.lock").is_dir());
    assert!(DirLock::acquire(&[d.join(".storage-write.lock")], 15_000, 1, Duration::ZERO).is_none());
    drop(a);
    assert!(!d.join(".storage-write.lock").exists());
    assert!(storage(&d).is_some());
}

#[test]
fn takes_over_an_abandoned_lock_only() {
    let d = dir("stale");
    let l = d.join("x.lock");
    fs::create_dir(&l).unwrap();
    assert!(DirLock::acquire(std::slice::from_ref(&l), 60_000, 1, Duration::ZERO).is_none(), "fresh: someone holds it");
    touch_at(&l, SystemTime::now() - Duration::from_secs(120));
    assert!(DirLock::acquire(&[l], 60_000, 1, Duration::ZERO).is_some(), "two minutes old: abandoned");
}

#[test]
fn keeps_its_lock_fresh_while_held() {
    let d = dir("touch");
    let l = d.join("y.lock");
    let held = DirLock::acquire(std::slice::from_ref(&l), 3_000, 1, Duration::ZERO).unwrap();
    touch_at(&l, SystemTime::now() - Duration::from_secs(60));
    thread::sleep(Duration::from_millis(2600));
    let age = SystemTime::now().duration_since(fs::metadata(&l).unwrap().modified().unwrap()).unwrap();
    assert!(age < Duration::from_secs(3), "touched within the stale time, age {age:?}");
    assert!(DirLock::acquire(&[l], 3_000, 1, Duration::ZERO).is_none());
    drop(held);
}

#[test]
fn all_or_none() {
    let d = dir("all");
    let (a, b) = (d.join("a.lock"), d.join("b.lock"));
    fs::create_dir(&b).unwrap();
    assert!(DirLock::acquire(&[a.clone(), b], 60_000, 1, Duration::ZERO).is_none());
    assert!(!a.exists(), "the part taken is given back");
}

#[test]
fn config_lock_sits_next_to_the_file() {
    let d = dir("cfg");
    let f = d.join(".claude.json");
    let _l = config(&f).unwrap();
    assert!(d.join(".claude.json.lock").is_dir());
}

fn touch_at(d: &Path, t: SystemTime) {
    let mut o = fs::OpenOptions::new();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.access_mode(0x100).custom_flags(0x0200_0000);
    }
    #[cfg(not(windows))]
    o.read(true);
    o.open(d).unwrap().set_modified(t).unwrap();
}
