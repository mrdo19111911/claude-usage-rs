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

// A row under "Other accounts", with what its switch buttons need.
#[derive(Clone, Default)]
pub(super) struct Other {
    pub(super) email: String,
    pub(super) usage: Option<Use>,
    // Its Claude CLI login (Act::Cli).
    pub(super) id: String,
    // Its saved Claude Desktop sign-in (Act::Desktop); None: never signed in to Desktop here.
    pub(super) desk_uuid: Option<String>,
    // In use there now: that side gets no button.
    pub(super) in_cli: bool,
    pub(super) in_desk: bool,
}

#[derive(Clone, Default)]
pub(super) struct Model {
    pub(super) now: i64,
    pub(super) cli: Side,
    // None: no Claude Desktop on this computer.
    pub(super) desk: Option<Side>,
    // A Claude Desktop switch is running: no Desktop buttons meanwhile.
    pub(super) desk_switching: bool,
    pub(super) others: Vec<Other>,
    // When the newest figures were fetched (any account): "· 2 min ago" in the header.
    pub(super) checked: Option<i64>,
}

pub(super) const FIVE_MS: i64 = 5 * 3_600_000;
pub(super) const WEEK_MS: i64 = 7 * 86_400_000;

// Share of a `len`-long window gone by `now`, 0..=1, from its reset; None without one.
pub(super) fn pace(reset: Option<i64>, now: i64, len: i64) -> Option<f32> {
    let r = reset.filter(|&r| r > now)?;
    Some((1.0 - (r - now) as f64 / len as f64).clamp(0.0, 1.0) as f32)
}

// A full account (a window at 100%): when it is usable again (the latest reset among its
// full windows; i64::MAX when one is unknown). None: not full.
pub(super) fn full_until(u: &Use) -> Option<i64> {
    [&u.five, &u.week].into_iter().filter(|w| w.u >= 100).map(|w| w.reset.unwrap_or(i64::MAX)).max()
}

// The "Switch to" rows (not full; most room first, no figures last) and the "Full · free
// again" rows (soonest free first, with that time).
pub(super) fn split_others(m: &Model) -> (Vec<&Other>, Vec<(&Other, i64)>) {
    let (mut open, mut full) = (vec![], vec![]);
    for o in &m.others {
        match o.usage.as_ref().and_then(full_until) {
            Some(t) => full.push((o, t)),
            None => open.push(o),
        }
    }
    let key = |o: &Other| o.usage.as_ref().map(|u| (u.five.u.max(u.week.u), u.week.u));
    open.sort_by(|a, b| (key(a).is_none(), key(a), &a.email).cmp(&(key(b).is_none(), key(b), &b.email)));
    full.sort_by(|a, b| (a.1, &a.0.email).cmp(&(b.1, &b.0.email)));
    (open, full)
}

// Claude CLI and Claude Desktop are on one account: one in-use card.
pub(super) fn same_account(m: &Model) -> bool {
    match (&m.cli.email, m.desk.as_ref().and_then(|d| d.email.as_ref())) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}

pub(super) fn usage(v: &Value) -> Option<Use> {
    let win = |w: &Value| Win { u: w["u"].as_i64().unwrap_or(0), reset: w["resetsAt"].as_i64() };
    v.is_object().then(|| Use { five: win(&v["five"]), week: win(&v["week"]) })
}

pub(super) fn model(state: &str) -> Model {
    let v: Value = serde_json::from_str(state).unwrap_or_default();
    let mut m = Model { now: v["now"].as_i64().unwrap_or(0), ..Default::default() };
    // Claude Desktop's accounts; none when it is not here or in demo.
    let d = &v["desktop"];
    let found = d["found"].as_bool() == Some(true) && v["demo"].as_bool() != Some(true);
    let desk: &[Value] = if found { d["accounts"].as_array().map_or(&[], |l| l.as_slice()) } else { &[] };
    let active = v["activeId"].as_str();
    for a in v["accounts"].as_array().into_iter().flatten() {
        let email = a["email"].as_str().unwrap_or("?").to_string();
        let uuid = a["uuid"].as_str();
        let entry = uuid.and_then(|u| desk.iter().find(|e| e["uuid"].as_str() == Some(u)));
        let desk_uuid = entry.filter(|e| e["hasSignIn"].as_bool() == Some(true)).and(uuid).map(String::from);
        let in_desk = entry.is_some_and(|e| e["signedIn"].as_bool() == Some(true));
        let in_cli = active.is_some() && a["id"].as_str() == active;
        if in_cli {
            m.cli = Side { email: Some(email.clone()), usage: usage(&a["usage"]) };
            // Also a row when Desktop could switch to it.
            if in_desk || desk_uuid.is_none() {
                continue;
            }
        }
        let id = a["id"].as_str().unwrap_or("").to_string();
        m.others.push(Other { email, usage: usage(&a["usage"]), id, desk_uuid, in_cli, in_desk });
    }
    if found {
        m.desk_switching = d["switching"].as_bool() == Some(true);
        let on = desk.iter().find(|a| a["signedIn"].as_bool() == Some(true));
        m.desk = Some(on.map_or_else(Side::default, |a| Side {
            email: a["email"].as_str().or(a["label"].as_str()).map(String::from),
            usage: usage(&a["usage"]),
        }));
    }
    // Newest figures: the latest fetch over every account, either side.
    let all = v["accounts"].as_array().into_iter().flatten().chain(d["accounts"].as_array().into_iter().flatten());
    m.checked = all.filter_map(|a| a["usage"]["fetchedAt"].as_i64()).max();
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
