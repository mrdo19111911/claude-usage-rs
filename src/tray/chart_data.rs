// The points of an open row's chart (docs/specs/gadget-chart.md): pure, no drawing.
// Mirrors the dashboard's winSpan / winPoints (src/web/js/chart.js); the per-column
// thinning is docs/SO_SANH_GADGET_CHART.md, "Downsampling".

use super::*;

// A window as the chart shows it: (%, reset still ahead, reset that has passed).
// A passed reset counts as 0 % (docs/specs/account-cards.md, "A reset that has passed").
pub(super) fn eff(w: &Win, now: i64) -> (i64, Option<i64>, Option<i64>) {
    match w.reset {
        Some(r) if r <= now => (0, None, Some(r)),
        r => (w.u, r, None),
    }
}

// From the window's start to its reset; without one ahead, the last `len` to now.
pub(super) fn span(reset: Option<i64>, now: i64, len: i64) -> (i64, i64) {
    match reset.filter(|&r| r > now) {
        Some(r) => (r - len, r),
        None => (now - len, now),
    }
}

// The vertical lines in (start, end]: each whole hour (5-hour), or each local midnight
// (week), as the dashboard's ticks(); `off` is this computer's offset from UTC in ms.
pub(super) fn ticks(start: i64, end: i64, week: bool, off: i64) -> Vec<i64> {
    let step = if week { 24 * 3_600_000 } else { 3_600_000 };
    let first = ((start + off).div_euclid(step) + 1) * step - off;
    (0..).map(|i| first + i * step).take_while(|&t| t <= end).collect()
}

// (time, %) of one window (week: the weekly column) in [start, now]: 0 at the start when
// older figures exist, the line cut at a passed reset `was` and dropped to 0 there, the
// current figure `u` at now.
pub(super) fn points(hist: &[(i64, i64, i64)], week: bool, start: i64, now: i64, u: Option<i64>, was: Option<i64>) -> Vec<(i64, i64)> {
    let cut = was.filter(|&w| w > start && w <= now).unwrap_or(i64::MAX);
    let pick = |p: &(i64, i64, i64)| if week { p.2 } else { p.1 };
    let mut out = vec![];
    if hist.first().is_some_and(|p| p.0 < start) {
        out.push((start, 0));
    }
    out.extend(hist.iter().filter(|p| p.0 >= start && p.0 <= now && p.0 < cut).map(|p| (p.0, pick(p))));
    if cut != i64::MAX {
        if let Some(&(_, last)) = out.last() {
            out.extend([(cut, last), (cut, 0)]);
        }
    }
    if let Some(u) = u {
        out.push((now, u));
    }
    out
}

// Pixels in the box (left, top, right, bottom): time across [start, end], 0-100 % up.
pub(super) fn to_xy(pts: &[(i64, i64)], start: i64, end: i64, b: (f32, f32, f32, f32)) -> Vec<(f32, f32)> {
    let (l, t, r, btm) = b;
    let len = (end - start).max(1) as f64;
    pts.iter()
        .map(|&(tm, u)| {
            let x = l + ((tm - start) as f64 / len) as f32 * (r - l);
            let y = btm - (u.clamp(0, 100) as f32 / 100.0) * (btm - t);
            (x.clamp(l, r), y)
        })
        .collect()
}

// At most the first, lowest, highest and last point of each pixel column, in order:
// keeps the spikes, caps the line at 4 points per column.
pub(super) fn thin(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = vec![];
    let mut i = 0;
    while i < pts.len() {
        let col = pts[i].0.floor();
        let j = i + pts[i..].iter().take_while(|p| p.0.floor() == col).count();
        let run = &pts[i..j];
        let lo = run.iter().enumerate().min_by(|a, b| a.1 .1.total_cmp(&b.1 .1)).map_or(0, |e| e.0);
        let hi = run.iter().enumerate().max_by(|a, b| a.1 .1.total_cmp(&b.1 .1)).map_or(0, |e| e.0);
        let mut keep = vec![0, lo, hi, run.len() - 1];
        keep.sort_unstable();
        keep.dedup();
        out.extend(keep.into_iter().map(|k| run[k]));
        i = j;
    }
    out
}

#[cfg(test)]
#[path = "chart_data_tests.rs"]
mod tests;
