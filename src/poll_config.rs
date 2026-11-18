// How often usage is asked for (docs/specs/check-schedule.md): the settings and the pure
// interval rules. No clock, no state: the RNG is passed in.

use crate::*;

const MIN: i64 = 60_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PollCfg {
    // The account in use: minutes between checks below mid_pct, from it, and from high_pct.
    pub(crate) low_min: u32,
    pub(crate) mid_min: u32,
    pub(crate) high_min: u32,
    pub(crate) mid_pct: u32,
    pub(crate) high_pct: u32,
    // The weekly % from which the account in use is checked at high_min; below it the week is ignored.
    pub(crate) week_high_pct: u32,
    // Other accounts: checked only when on, each at a random time in [idle_min, idle_max].
    pub(crate) poll_others: bool,
    pub(crate) idle_min: u32,
    pub(crate) idle_max: u32,
}

impl Default for PollCfg {
    fn default() -> Self {
        PollCfg { low_min: 15, mid_min: 7, high_min: 2, mid_pct: 50, high_pct: 90, week_high_pct: 98, poll_others: false, idle_min: 10, idle_max: 15 }
    }
}

impl PollCfg {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let mins = [self.low_min, self.mid_min, self.high_min, self.idle_min, self.idle_max];
        if mins.iter().any(|m| !(1..=120).contains(m)) {
            return Err("Check intervals must be between 1 and 120 minutes");
        }
        if [self.mid_pct, self.high_pct, self.week_high_pct].iter().any(|p| !(1..=100).contains(p)) {
            return Err("Percentages must be between 1 and 100");
        }
        if self.mid_pct >= self.high_pct {
            return Err("The middle percentage must be lower than the high one");
        }
        if self.idle_min > self.idle_max {
            return Err("The shortest wait for other accounts must not exceed the longest");
        }
        Ok(())
    }

    // Takes the polling keys present in a settings request; all or nothing.
    pub(crate) fn apply_json(&mut self, body: &Value) -> Result<(), &'static str> {
        let mut next = self.clone();
        let num = |k: &str| body[k].as_f64().map(|v| v.clamp(0.0, 1000.0) as u32);
        for (key, field) in [("lowMin", &mut next.low_min), ("midMin", &mut next.mid_min), ("highMin", &mut next.high_min), ("midPct", &mut next.mid_pct), ("highPct", &mut next.high_pct), ("weekHighPct", &mut next.week_high_pct),("idleMin", &mut next.idle_min), ("idleMax", &mut next.idle_max)] {
            if let Some(v) = num(key) {
                *field = v;
            }
        }
        if let Some(b) = body["pollOthers"].as_bool() {
            next.poll_others = b;
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
}

// The wait (ms) after figures showing `five` and `week` %: tiered by the 5-hour % alone; the
// weekly % only counts from week_high_pct, as the high tier.
pub(crate) fn interval_for(five: i64, week: i64, c: &PollCfg) -> i64 {
    let m = if five >= c.high_pct as i64 || week >= c.week_high_pct as i64 {
        c.high_min
    } else if five >= c.mid_pct as i64 {
        c.mid_min
    } else {
        c.low_min
    };
    m as i64 * MIN
}

// A random wait (ms) for an account not in use, uniform in [idle_min, idle_max] minutes.
pub(crate) fn idle_interval(c: &PollCfg, rng: &mut dyn FnMut() -> u64) -> i64 {
    let (lo, hi) = (c.idle_min as i64 * MIN, c.idle_max.max(c.idle_min) as i64 * MIN);
    // A fraction of the range from the draw's top 53 bits.
    let f = (rng() >> 11) as f64 / ((1u64 << 53) - 1) as f64;
    lo + ((hi - lo) as f64 * f).round() as i64
}

// A tiny xorshift64*: enough to spread checks; not for secrets.
pub(crate) struct XorShift(u64);

impl XorShift {
    pub(crate) fn new(seed: u64) -> Self {
        XorShift(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    // Seeded from the clock's nanoseconds and `salt` (e.g. an account id's hash).
    pub(crate) fn from_clock(salt: u64) -> Self {
        let ns = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        XorShift::new(ns ^ salt.rotate_left(29))
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}
