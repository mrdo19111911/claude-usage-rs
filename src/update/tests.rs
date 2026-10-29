// Tests of the pure parts of updating (docs/specs/update.md, "Tests first").

use super::install::setup_args;
use super::*;

#[test]
fn version_is_cargos() {
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn newer_compares_part_by_part() {
    assert!(newer("1.10.0", "1.9.0"), "numbers, not text");
    assert!(newer("v1.2.0", "1.1.0"), "a leading v is dropped");
    assert!(newer("2.0", "1.9.9"));
    assert!(newer("1.1.1", "v1.1"));
    assert!(!newer("1.1.0", "1.1.0"), "equal is not newer");
    assert!(!newer("1.1", "1.1.0"), "a missing part counts as 0");
    assert!(!newer("1.0.9", "1.1.0"));
    assert!(!newer("latest", "1.1.0"), "bad text is not newer");
    assert!(!newer("1.2.x", "1.1.0"));
    assert!(!newer("+2.0.0", "1.1.0"), "digits only");
    assert!(!newer("", "1.1.0"));
    assert!(!newer("1..2", "1.1.0"));
}

fn release(digest: Value) -> Value {
    json!({
        "tag_name": "v1.2.0",
        "html_url": "https://github.com/mrdo19111911/claude-usage-rs/releases/tag/v1.2.0",
        "assets": [
            {"name": "claude-usage.exe", "size": 10, "digest": format!("sha256:{}", "a".repeat(64)),
             "browser_download_url": "https://github.com/mrdo19111911/claude-usage-rs/releases/download/v1.2.0/claude-usage.exe"},
            {"name": "ClaudeUsage-Setup-1.2.0.exe", "size": 1234, "digest": digest,
             "browser_download_url": "https://github.com/mrdo19111911/claude-usage-rs/releases/download/v1.2.0/ClaudeUsage-Setup-1.2.0.exe"}
        ]
    })
}

#[test]
fn picks_the_setup_by_name() {
    let hex = "AB".repeat(32);
    let a = pick_asset(&release(json!(format!("sha256:{hex}")))).unwrap();
    assert_eq!(a.name, "ClaudeUsage-Setup-1.2.0.exe");
    assert_eq!(a.version, "1.2.0");
    assert_eq!(a.size, 1234);
    assert_eq!(a.sha256, "ab".repeat(32), "kept lower case, as sha256_hex writes it");
    assert!(a.url.ends_with("/v1.2.0/ClaudeUsage-Setup-1.2.0.exe"));
}

#[test]
fn a_setup_without_a_good_digest_is_not_installable() {
    let e = pick_asset(&release(Value::Null)).unwrap_err();
    assert!(e.contains("digest"), "{e}");
    for bad in ["sha256:abc", "md5:0123", &format!("sha256:{}", "g".repeat(64)), &format!("sha512:{}", "a".repeat(64))] {
        assert!(pick_asset(&release(json!(bad))).is_err(), "{bad}");
    }
}

#[test]
fn pick_asset_refuses_odd_releases() {
    let good = json!(format!("sha256:{}", "0".repeat(64)));
    let mut r = release(good.clone());
    r["tag_name"] = json!("v1.2.0/../../x");
    assert!(pick_asset(&r).is_err(), "the tag becomes a file name: versions only");
    let mut r = release(good.clone());
    r["assets"][1]["browser_download_url"] = json!("https://evil.example/ClaudeUsage-Setup-1.2.0.exe");
    assert!(pick_asset(&r).is_err(), "only github.com downloads");
    let mut r = release(good.clone());
    r["assets"][1]["name"] = json!("ClaudeUsage-Setup-1.1.0.exe");
    assert!(pick_asset(&r).unwrap_err().contains("ClaudeUsage-Setup-1.2.0.exe"), "says what is missing");
    let mut r = release(good);
    r["assets"][1]["size"] = json!(0);
    assert!(pick_asset(&r).is_err(), "no size: cannot check the download");
}

#[test]
fn sha256_known_value() {
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
}

#[test]
fn downloads_only_from_github_hosts() {
    assert!(download_host_ok("https://github.com/o/r/releases/download/v1/x.exe"));
    assert!(download_host_ok("https://objects.githubusercontent.com/github-production-release-asset/1?x=y"));
    assert!(download_host_ok("https://release-assets.githubusercontent.com/github-production-release-asset/1"));
    assert!(!download_host_ok("http://github.com/o/r/x.exe"), "https only");
    assert!(!download_host_ok("https://github.com.evil.example/x.exe"));
    assert!(!download_host_ok("https://evilgithubusercontent.com/x.exe"));
    assert!(!download_host_ok("https://user@evil.example/?https://github.com/"));
}

#[test]
fn settings_without_auto_update_read_as_on() {
    let s: Settings = serde_json::from_value(json!({"threshold": 90, "autoSwitch": false, "pollSeconds": 60})).unwrap();
    assert!(s.auto_update);
    assert!(State::default().settings.auto_update);
    let s: Settings = serde_json::from_value(json!({"threshold": 90, "autoSwitch": false, "pollSeconds": 60, "autoUpdate": false})).unwrap();
    assert!(!s.auto_update);
    assert_eq!(serde_json::to_value(&s).unwrap()["autoUpdate"], json!(false));
}

#[test]
fn status_json_matches_the_spec() {
    let v = serde_json::to_value(Status::default()).unwrap();
    assert_eq!(v, json!({"status": "idle", "latest": null, "url": null, "checkedAt": null, "error": null}));
    let s = Status { phase: Phase::UpToDate, checked_at: Some(5), ..Default::default() };
    assert_eq!(serde_json::to_value(&s).unwrap()["status"], "upToDate");
    for (p, name) in [(Phase::Checking, "checking"), (Phase::Available, "available"), (Phase::Downloading, "downloading"), (Phase::Installing, "installing"), (Phase::Error, "error")] {
        let s = Status { phase: p, ..Default::default() };
        assert_eq!(serde_json::to_value(&s).unwrap()["status"], name);
    }
}

#[test]
fn setup_runs_silently_and_relaunches() {
    let dir = std::path::Path::new(r"C:\Temp\claude-usage-update");
    let args = setup_args(dir);
    assert_eq!(args[..5], ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/RELAUNCH=1"]);
    #[cfg(windows)]
    assert_eq!(args[5], r"/LOG=C:\Temp\claude-usage-update\setup.log");
    assert_eq!(args.len(), 6);
}

#[test]
fn checks_by_itself_every_3_days() {
    let day = 24 * 3_600_000;
    assert!(check_due(None, 10 * day), "never checked: now");
    assert!(!check_due(Some(10 * day), 10 * day + 2 * day), "2 days later: not yet");
    assert!(check_due(Some(10 * day), 13 * day), "3 days later: due");
    assert!(!check_due(Some(10 * day), 9 * day), "a clock set back: wait");
}

#[test]
fn saved_state_without_a_check_time_reads() {
    let st: State = serde_json::from_str(r#"{"settings":{"threshold":95,"autoSwitch":true,"pollSeconds":120},"accounts":[],"activeId":null}"#).unwrap();
    assert_eq!(st.update_checked_at, None);
}
