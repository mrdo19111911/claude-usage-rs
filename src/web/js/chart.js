// An account card's two charts (docs/specs/account-cards.md): its 5-hour and weekly
// windows, each from the window's start to its reset (a green line, the right edge), with
// even pace and now, and on the 5-hour one the pace of the last 20 min.
// Parts: winSpan, winPoints, ticks, pace, paceTip, winChart, hovering, charts, mousemove /
// mouseleave handlers on #cards and #sides (the "In use now" panels show the same panes).

// The window shown, [start, end]: a known reset, from reset - span to the reset; not
// started (or the reset passed): the last `span` up to now.
function winSpan(w, span, now) {
  return w && w.resetsAt > now ? [w.resetsAt - span, w.resetsAt] : [now - span, now];
}

// [time, %] in the window up to now (k: 1 = 5-hour, 2 = weekly). A window starts at 0%;
// the figures hold until the next check, so the last one is carried to now.
function winPoints(a, k, start, now, u) {
  const pts = a.history.filter(p => p[0] >= start && p[0] <= now).map(p => [p[0], p[k]]);
  if (a.history.length && a.history[0][0] < start) pts.unshift([start, 0]);
  if (u != null) pts.push([now, u]);
  return pts;
}

// Axis ticks: each whole hour (5-hour), or each midnight labelled day-month ("3-11").
function ticks(start, end, week) {
  const out = [], d = new Date(start);
  if (week) d.setHours(24, 0, 0, 0); else d.setMinutes(60, 0, 0);
  for (; +d <= end; week ? d.setDate(d.getDate() + 1) : d.setHours(d.getHours() + 1))
    out.push([+d, week ? `${d.getDate()}-${d.getMonth() + 1}` : hhmm(+d)]);
  return out;
}

// The pace now (docs/specs/account-cards.md, "Pace line"), in % per ms: the slope of a
// least-squares line through this window's checks of the last 20 min (k: 1 = 5-hour);
// null with under 2 checks or when not rising. The % is whole numbers: 2 checks alone
// mostly differ by 0. Also null when it has not moved for the last 10 min: use has
// paused, so a pace from before it would be stale.
const PACE_MS = 20 * 60000, FLAT_MS = 10 * 60000;
function pace(history, k, start, now) {
  const pts = history.filter(p => p[0] >= Math.max(start, now - PACE_MS) && p[0] <= now);
  const flat = pts.filter(p => p[0] >= now - FLAT_MS);
  if (flat.length >= 2 && flat.every(p => p[k] === flat[0][k])) return null;
  const n = pts.length, mt = pts.reduce((a, p) => a + p[0], 0) / n, mu = pts.reduce((a, p) => a + p[k], 0) / n;
  const d = pts.reduce((a, p) => a + (p[0] - mt) ** 2, 0);
  if (pts.length < 2 || !d) return null;
  const s = pts.reduce((a, p) => a + (p[0] - mt) * (p[k] - mu), 0) / d;
  return s > 0 ? s : null;
}

// The pace line's tooltip (to the right of now): when the limit is expected at that pace,
// or the % expected at the reset when it does not get there first.
function paceTip(f, col) {
  const m = Math.round((f.te - f.now) / 60000), inm = m >= 60 ? `${Math.floor(m / 60)}h${String(m % 60).padStart(2, '0')}m` : `${m} min`;
  return `<div style="color:var(--muted)">At the last 20 min's pace</div>
    <div><i style="background:${col}"></i>${f.te < f.end ? `Expected limit at ${hhmm(f.te)}, in ${inm}` : `Expected ~${Math.round(f.ue)}% at reset (${hhmm(f.end)})`}</div>`;
}

function winChart(a, k, now) {
  const week = k === 2, span = week ? 7 * 24 * H : 5 * H;
  const w = a.usage && effWin(week ? a.usage.week : a.usage.five);
  const [start, end] = winSpan(w, span, now);
  const lim = week ? (S.settings.weekThreshold ?? 99) : S.settings.threshold;
  const col = week ? 'var(--s-week)' : 'var(--s-five)';
  const pts = winPoints(a, k, start, now, w && w.u);
  const W = 300, Ht = 104, L = 24, R = 10, T = 13, B = 18;
  const x = t => L + (t - start) / (end - start) * (W - L - R);
  const y = u => T + (1 - u / 100) * (Ht - T - B);
  let g = '', proj = null;
  for (const v of [0, 50, 100]) g += `<line x1="${L}" x2="${W - R}" y1="${y(v)}" y2="${y(v)}" stroke="var(--grid)"/><text x="${L - 6}" y="${y(v) + 4}" text-anchor="end">${v}</text>`;
  // The reset, always the right edge when known: its time goes under the axis, so a tick
  // label that would sit under it is left out (its column stays).
  const rx = x(end), hasReset = end > now, day = t => new Date(t).toLocaleDateString('en-US', { weekday: 'short' });
  const rlab = week ? `${day(end)} ${hhmm(end)}` : hhmm(end), rw = hasReset ? rlab.length * 6.5 + 20 : 0;
  for (const [t, lab] of ticks(start, end, week))
    g += `<line x1="${x(t)}" x2="${x(t)}" y1="${T}" y2="${y(0)}" stroke="var(--grid)"/>${x(t) < rx - rw ? `<text x="${x(t)}" y="${Ht - 5}" text-anchor="middle">${lab}</text>` : ''}`;
  if (hasReset) {
    g += `<line x1="${rx}" x2="${rx}" y1="${T - 4}" y2="${y(0)}" stroke="var(--green)" stroke-width="2"/>`;
    g += `<text x="${W - 1}" y="${T - 4}" text-anchor="end" style="fill:var(--green)">reset</text>`;
    g += `<text x="${W - 1}" y="${Ht - 5}" text-anchor="end" style="fill:var(--green);font-weight:600">${rlab}</text>`;
  }
  // The auto-switch line of this window, in its colour.
  g += `<line x1="${L}" x2="${W - R}" y1="${y(lim)}" y2="${y(lim)}" stroke="${col}" stroke-opacity=".55" stroke-dasharray="4 4"/>`;
  if (!w) {
    g += `<text x="${(L + W - R) / 2}" y="${y(50)}" text-anchor="middle">No figures yet</text>`;
  } else {
    if (end > now) {
      // Even pace: 0% at the window's start to 100% at its reset. Above it, the window
      // runs out before it resets; below it, some of it is left unused at the reset.
      g += `<line x1="${x(start)}" x2="${x(end)}" y1="${y(0)}" y2="${y(100)}" stroke="var(--muted)" stroke-opacity=".7" stroke-dasharray="2 4"/>`;
      const deg = Math.atan2(y(100) - y(0), x(end) - x(start)) * 180 / Math.PI;
      g += `<text transform="translate(${x(start + span / 2)},${y(50)}) rotate(${deg.toFixed(1)})" dy="-5" text-anchor="middle">even pace</text>`;
      g += `<line x1="${x(now)}" x2="${x(now)}" y1="${T - 4}" y2="${y(0)}" stroke="var(--red)" stroke-dasharray="4 3" stroke-width="1.5"/>`;
      // Near the reset, "now" moves left so it does not cover "reset".
      g += `<text x="${Math.min(x(now), rx - 40)}" y="${T - 4}" text-anchor="middle" style="fill:var(--red)">now</text>`;
    }
    // Pace line: from the current point at the last 20 min's pace to 100%, or to the reset
    // if it does not get there first (a 20-min pace says nothing about a week).
    if (!week && end > now && w.u < 100) {
      const s = pace(a.history, k, start, now);
      if (s) {
        const te = Math.min(now + (100 - w.u) / s, end), ue = w.u + s * (te - now);
        // A short label; the tooltip says it in full.
        const lab = te < end ? `~${hhmm(te)}` : `~${Math.round(ue)}%`;
        proj = { now, te, ue, end };
        g += `<line x1="${x(now)}" y1="${y(w.u)}" x2="${x(te)}" y2="${y(ue)}" stroke="${col}" stroke-width="1.5" stroke-dasharray="1 3" stroke-linecap="round"/>`;
        // The label right of the line's end when it fits before the reset, else under it.
        const ex = x(te), right = ex + 6 + lab.length * 6.5 < rx;
        g += `<text x="${right ? ex + 6 : ex - 4}" y="${y(ue) + (right ? 4 : ue > 50 ? 12 : -6)}" text-anchor="${right ? 'start' : 'end'}" style="fill:${col}">${lab}</text>`;
      }
    }
    // Anthropic gives only the figures now: the line starts when recording did.
    const first = a.history[0];
    if (first && first[0] > start && first[0] < now) {
      const fx = x(first[0]), right = fx > (L + W - R) / 2;
      g += `<text x="${fx + (right ? -6 : 6)}" y="${y(70)}" text-anchor="${right ? 'end' : 'start'}">Recording since ${hhmm(first[0])}</text>`;
    }
    if (pts.length > 1) g += `<path d="${pts.map((p, i) => `${i ? 'L' : 'M'}${x(p[0]).toFixed(1)},${y(p[1]).toFixed(1)}`).join('')}" fill="none" stroke="${col}" stroke-width="2" stroke-linejoin="round"/>`;
    const nx = x(now), left = nx > W - R - 40;
    g += `<circle cx="${nx}" cy="${y(w.u)}" r="5" fill="${col}" stroke="var(--card)" stroke-width="2"/>`;
    g += `<text x="${nx + (left ? -9 : 9)}" y="${y(w.u) + (w.u > 85 ? 16 : -6)}" text-anchor="${left ? 'end' : 'start'}" style="fill:var(--text)">${w.u}%</text>`;
  }
  g += `<line class="cross" x1="0" x2="0" y1="${T}" y2="${y(0)}" stroke="#777" style="display:none"/>`;
  g += `<rect x="${L}" y="0" width="${W - L - R}" height="${Ht}" fill="transparent"/>`;
  return { svg: `<svg viewBox="0 0 ${W} ${Ht}" data-key="${a.id}:${k}" role="img" aria-label="${week ? 'Weekly' : '5-hour'} limit used over its window">${g}</svg>`, pts, x, W, week, col, proj };
}

// A redraw would drop the tooltip under the pointer: cards and panels wait while it is on a chart.
let hovering = false, charts = {};
// Crosshair and tooltip: the figure in force at the pointer's time.
for (const root of [$('#cards'), $('#sides')]) {
root.addEventListener('mousemove', e => {
  const svg = e.target.closest('svg[data-key]');
  hovering = !!svg;
  root.querySelectorAll('.tip').forEach(t => { if (!svg || t.parentNode !== svg.parentNode) t.style.display = 'none'; });
  root.querySelectorAll('.cross').forEach(l => { if (!svg || l.ownerSVGElement !== svg) l.style.display = 'none'; });
  if (!svg) return;
  const c = charts[svg.dataset.key];
  if (!c || c.pts.length < 2) return;
  const box = svg.getBoundingClientRect(), sx = (e.clientX - box.left) / box.width * c.W;
  // Right of now on a chart with a pace line: what that line expects.
  const f = c.proj && sx > c.x(c.proj.now) ? c.proj : null;
  let p = c.pts[0];
  for (const q of c.pts) if (c.x(q[0]) <= sx) p = q;
  const cross = svg.querySelector('.cross'), cx = f ? c.x(f.te) : c.x(p[0]);
  cross.setAttribute('x1', cx); cross.setAttribute('x2', cx); cross.style.display = '';
  const tip = svg.parentNode.querySelector('.tip');
  tip.innerHTML = f ? paceTip(f, c.col) : `<div style="color:var(--muted)">${c.week ? new Date(p[0]).toLocaleDateString('en-US', { weekday: 'short' }) + ' ' : ''}${hhmm(p[0])}</div>
    <div><i style="background:${c.col}"></i>${c.week ? 'Weekly' : '5-hour'} ${p[1]}%</div>`;
  tip.style.display = 'block';
  const host = svg.parentNode.getBoundingClientRect();
  tip.style.left = Math.min(e.clientX - host.left + 14, host.width - tip.offsetWidth - 6) + 'px';
  tip.style.top = (box.top - host.top + 4) + 'px';
});
root.addEventListener('mouseleave', () => {
  hovering = false;
  root.querySelectorAll('.tip').forEach(t => (t.style.display = 'none'));
  root.querySelectorAll('.cross').forEach(l => (l.style.display = 'none'));
});
}
