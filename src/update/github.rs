// The latest release on GitHub and its setup: pure parts tested in tests.rs.

use super::*;

const LATEST: &str = "https://api.github.com/repos/mrdo19111911/claude-usage-rs/releases/latest";

// The setup a release carries, as the API describes it.
#[derive(Debug, Clone)]
pub(crate) struct Asset {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) size: u64,
    // Lower-case hex, as sha256_hex writes it.
    pub(crate) sha256: String,
    pub(crate) version: String,
}

// "1.10.0" -> [1, 10, 0]; a leading "v" is dropped. Digits only: the version also
// names the setup file, so nothing else may get through.
fn parts(v: &str) -> Option<Vec<u64>> {
    let v = v.strip_prefix('v').unwrap_or(v);
    v.split('.')
        .map(|p| if !p.is_empty() && p.len() < 10 && p.bytes().all(|b| b.is_ascii_digit()) { p.parse().ok() } else { None })
        .collect()
}

// Whether `latest` is a higher version than `current`, part by part (a missing part is
// 0). Text that is not a version is never newer.
pub(crate) fn newer(latest: &str, current: &str) -> bool {
    let (Some(mut a), Some(mut b)) = (parts(latest), parts(current)) else { return false };
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

// The release's setup, ready to download and check. No usable digest: not installable,
// since the download could not be checked.
pub(crate) fn pick_asset(release: &Value) -> Result<Asset, String> {
    let tag = release["tag_name"].as_str().unwrap_or("");
    if parts(tag).is_none() {
        return Err(format!("The latest release's tag ({tag:?}) is not a version."));
    }
    let version = tag.trim_start_matches('v').to_string();
    let name = format!("ClaudeUsage-Setup-{version}.exe");
    let a = release["assets"]
        .as_array()
        .and_then(|l| l.iter().find(|a| a["name"].as_str() == Some(name.as_str())))
        .ok_or_else(|| format!("Release {version} has no {name}; install it by hand from the release page."))?;
    let url = a["browser_download_url"].as_str().filter(|u| u.starts_with("https://github.com/"));
    let url = url.ok_or_else(|| format!("{name} is not on github.com."))?.to_string();
    let size = a["size"].as_u64().filter(|&s| s > 0).ok_or_else(|| format!("GitHub gives no size for {name}."))?;
    let hex = a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:"));
    let hex = hex.filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()));
    let sha256 = hex
        .ok_or_else(|| format!("GitHub gives no SHA-256 digest for {name}, so the download cannot be checked; install it by hand from the release page."))?
        .to_ascii_lowercase();
    Ok(Asset { name, url, size, sha256, version })
}

// The text the release's digest is compared with (install.rs hashes as it streams, to
// the same text through `hex`).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    hex(&sha2::Sha256::digest(bytes))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Where the download may end up after GitHub's redirect: github.com or its asset hosts
// (objects. / release-assets.githubusercontent.com), https only. The digest is the real
// check; this keeps a stray redirect from fetching anything else at all.
pub(crate) fn download_host_ok(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    host == "github.com" || host.strip_suffix(".githubusercontent.com").is_some_and(|sub| !sub.is_empty() && !sub.contains(['@', ':']))
}

pub(crate) fn user_agent() -> String {
    format!("claude-usage/{VERSION}")
}

// Drafts and pre-releases are never "latest" for this endpoint.
pub(crate) fn latest_release(http: &ureq::Agent) -> Result<Value, String> {
    let resp = http
        .get(LATEST)
        .set("User-Agent", &user_agent())
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "No release is published yet.".to_string(),
            ureq::Error::Status(c, _) => format!("GitHub answered HTTP {c}."),
            ureq::Error::Transport(t) => format!("Could not reach GitHub: {t}"),
        })?;
    resp.into_json::<Value>().map_err(|e| format!("GitHub's answer could not be read: {e}"))
}
