// The feed's token and the pairing text a phone scans (docs/specs/mobile-feed.md).
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::path::Path;

pub(crate) const TOKEN_FILE: &str = "mobile-token";
pub(crate) const SCHEME: &str = "scas://pair?";

pub(crate) struct Pairing {
    pub host: String,
    pub port: u16,
    pub token: String,
}

/// 32 random bytes, base64url without padding: 43 characters.
pub(crate) fn new_token() -> String {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).expect("system random numbers");
    URL_SAFE_NO_PAD.encode(b)
}

/// The token in `dir/mobile-token`, made on first use. Deleting the file unpairs phones.
pub(crate) fn load_or_create(dir: &Path) -> std::io::Result<String> {
    let f = dir.join(TOKEN_FILE);
    if let Ok(t) = std::fs::read_to_string(&f) {
        let t = t.trim();
        if t.len() >= 32 {
            return Ok(t.to_string());
        }
    }
    let t = new_token();
    std::fs::write(&f, &t)?;
    Ok(t)
}

pub(crate) fn url(host: &str, port: u16, token: &str) -> String {
    let e = crate::login::encode;
    format!("{SCHEME}host={}&port={port}&token={}", e(host), e(token))
}

pub(crate) fn parse(s: &str) -> Option<Pairing> {
    let s = s.trim();
    if !s.starts_with(SCHEME) {
        return None;
    }
    let p = |k| crate::login::param(s, k).filter(|v| !v.is_empty());
    let port = p("port")?.parse::<u16>().ok().filter(|&n| n > 0)?;
    Some(Pairing { host: p("host")?, port, token: p("token")? })
}

/// Equal strings, in time that does not depend on where they differ.
pub(crate) fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8 | u8::from(a.len() != b.len());
    for i in 0..a.len().max(b.len()) {
        diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    diff == 0
}

#[cfg(test)]
#[path = "pair_tests.rs"]
mod tests;
