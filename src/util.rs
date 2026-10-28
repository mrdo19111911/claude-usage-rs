// Small helpers: time, files, dates, the browser.

use crate::*;

pub(crate) fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

// ---------- helpers ----------
pub(crate) fn write_private(path: &Path, data: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("cud.tmp");
    {
        let mut o = fs::OpenOptions::new();
        o.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        use std::io::Write;
        o.open(&tmp)?.write_all(data.as_bytes())?;
    }
    fs::rename(&tmp, path)
}

pub(crate) fn to_win(w: &Value) -> Win {
    Win {
        u: w.get("utilization").and_then(Value::as_f64).unwrap_or(0.0).round() as i64,
        resets_at: w.get("resets_at").and_then(Value::as_str).and_then(parse_iso),
    }
}

// "2025-10-28T11:40:00.123+00:00" -> epoch ms
pub(crate) fn parse_iso(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let n = |a: usize, l: usize| -> Option<i64> { s.get(a..a + l)?.parse().ok() };
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(5, 2)?, n(8, 2)?, n(11, 2)?, n(14, 2)?, n(17, 2)?);
    let mut i = 19;
    let mut ms = 0;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let frac = &s[start..(start + 3).min(i)];
        ms = format!("{frac:0<3}").parse().ok()?;
    }
    let mut off = 0;
    if let Some(&c) = b.get(i) {
        if c == b'+' || c == b'-' {
            off = (n(i + 1, 2)? * 60 + n(i + 4, 2)?) * if c == b'+' { 1 } else { -1 };
        }
    }
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((mo + 9) % 12) + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Some((days * 86_400 + h * 3600 + mi * 60 + se - off * 60) * 1000 + ms)
}

pub(crate) fn home() -> PathBuf {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_else(|| ".".into())
}


pub(crate) fn open_browser(url: &str) {
    let _ = if cfg!(windows) {
        // Not `cmd /C start`: cmd would cut the URL at its first '&'.
        Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
}

