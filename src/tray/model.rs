// What the widget shows, read from the dashboard's state; times as text.

use super::*;

// ---------- What the widget shows ----------

#[derive(Clone, Default)]
pub(super) struct Win {
    pub(super) u: i64,
    pub(super) reset: Option<i64>,
}

#[derive(Clone, Default)]
pub(super) struct Use {
    pub(super) five: Win,
    pub(super) week: Win,
}

#[derive(Clone, Default)]
pub(super) struct Side {
    pub(super) email: Option<String>,
    pub(super) usage: Option<Use>,
}

#[derive(Clone, Default)]
pub(super) struct Model {
    pub(super) now: i64,
    pub(super) threshold: i64,
    pub(super) cli: Side,
    // None: no Claude Desktop on this computer.
    pub(super) desk: Option<Side>,
    pub(super) others: Vec<(String, Option<Use>)>,
}

pub(super) fn usage(v: &Value) -> Option<Use> {
    let win = |w: &Value| Win { u: w["u"].as_i64().unwrap_or(0), reset: w["resetsAt"].as_i64() };
    v.is_object().then(|| Use { five: win(&v["five"]), week: win(&v["week"]) })
}

pub(super) fn model(state: &str) -> Model {
    let v: Value = serde_json::from_str(state).unwrap_or_default();
    let mut m = Model { now: v["now"].as_i64().unwrap_or(0), threshold: v["settings"]["threshold"].as_i64().unwrap_or(95), ..Default::default() };
    let active = v["activeId"].as_str();
    for a in v["accounts"].as_array().into_iter().flatten() {
        let email = a["email"].as_str().unwrap_or("?").to_string();
        if active.is_some() && a["id"].as_str() == active {
            m.cli = Side { email: Some(email), usage: usage(&a["usage"]) };
        } else {
            m.others.push((email, usage(&a["usage"])));
        }
    }
    let d = &v["desktop"];
    if d["found"].as_bool() == Some(true) && v["demo"].as_bool() != Some(true) {
        let on = d["accounts"].as_array().and_then(|l| l.iter().find(|a| a["signedIn"].as_bool() == Some(true)));
        m.desk = Some(on.map_or_else(Side::default, |a| Side {
            email: a["email"].as_str().or(a["label"].as_str()).map(String::from),
            usage: usage(&a["usage"]),
        }));
    }
    m
}

// A moment (ms since 1970) on this computer's clock: "22:20" today, else "Thu 11:00".
pub(super) fn clock(t: i64, now: i64) -> String {
    let local = |ms: i64| unsafe {
        let ft = (ms.max(0) as u64 + 11_644_473_600_000) * 10_000;
        let ft = FILETIME { dwLowDateTime: ft as u32, dwHighDateTime: (ft >> 32) as u32 };
        let (mut utc, mut loc): (SYSTEMTIME, SYSTEMTIME) = (mem::zeroed(), mem::zeroed());
        FileTimeToSystemTime(&ft, &mut utc);
        SystemTimeToTzSpecificLocalTime(ptr::null(), &utc, &mut loc);
        loc
    };
    let (a, b) = (local(t), local(now));
    let hm = format!("{:02}:{:02}", a.wHour, a.wMinute);
    if (a.wYear, a.wMonth, a.wDay) == (b.wYear, b.wMonth, b.wDay) {
        hm
    } else {
        format!("{} {hm}", ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][a.wDayOfWeek as usize % 7])
    }
}

// "4h39m", "2d 3h", "12m".
pub(super) fn left(ms: i64) -> String {
    let m = (ms.max(0) + 59_999) / 60_000;
    if m >= 1440 { format!("{}d {}h", m / 1440, m % 1440 / 60) } else if m >= 60 { format!("{}h{:02}m", m / 60, m % 60) } else { format!("{m}m") }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_both_sides_from_the_state() {
        let st = r#"{"now":1000,"settings":{"threshold":94},"activeId":"b","demo":false,
            "accounts":[{"id":"a","email":"a@x.com","usage":{"five":{"u":10,"resetsAt":2000},"week":{"u":100,"resetsAt":9000}}},
                        {"id":"b","email":"b@x.com","usage":null}],
            "desktop":{"found":true,"accounts":[{"label":"Account 1","email":null,"signedIn":true,"usage":null}]}}"#;
        let m = super::model(st);
        assert_eq!(m.cli.email.as_deref(), Some("b@x.com"));
        assert!(m.cli.usage.is_none());
        assert_eq!(m.desk.unwrap().email.as_deref(), Some("Account 1"));
        assert_eq!(m.others.len(), 1);
        assert_eq!(m.others[0].1.as_ref().unwrap().week.u, 100);
    }

    #[test]
    fn time_left() {
        assert_eq!(super::left(4 * 3_600_000 + 39 * 60_000), "4h39m");
        assert_eq!(super::left(51 * 3_600_000), "2d 3h");
        assert_eq!(super::left(30_000), "1m");
    }
}
