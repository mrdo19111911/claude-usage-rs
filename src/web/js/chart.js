// An account card's two charts (docs/specs/account-cards.md): its 5-hour and weekly
// windows, each from the window's start to its reset, with even pace and now.
// Parts: winSpan, winPoints, ticks, winChart, hovering, charts, #cards mousemove /
// mouseleave handlers.

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
  let g = '';
  for (const v of [0, 50, 100]) g += `<line x1="${L}" x2="${W - R}" y1="${y(v)}" y2="${y(v)}" stroke="var(--grid)"/><text x="${L - 6}" y="${y(v) + 4}" text-anchor="end">${v}</text>`;
  for (const [t, lab] of ticks(start, end, week))
    g += `<line x1="${x(t)}" x2="${x(t)}" y1="${T}" y2="${y(0)}" stroke="var(--grid)"/><text x="${x(t)}" y="${Ht - 5}" text-anchor="middle">${lab}</text>`;
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
      g += `<text x="${x(now)}" y="${T - 4}" text-anchor="middle" style="fill:var(--red)">now</text>`;
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
  return { svg: `<svg viewBox="0 0 ${W} ${Ht}" data-key="${a.id}:${k}" role="img" aria-label="${week ? 'Weekly' : '5-hour'} limit used over its window">${g}</svg>`, pts, x, W, week, col };
}

// A redraw would drop the tooltip under the pointer: cards wait while it is on a chart.
let hovering = false, charts = {};
// Crosshair and tooltip: the figure in force at the pointer's time.
$('#cards').addEventListener('mousemove', e => {
  const svg = e.target.closest('svg[data-key]');
  hovering = !!svg;
  document.querySelectorAll('#cards .tip').forEach(t => { if (!svg || t.parentNode !== svg.parentNode) t.style.display = 'none'; });
  document.querySelectorAll('#cards .cross').forEach(l => { if (!svg || l.ownerSVGElement !== svg) l.style.display = 'none'; });
  if (!svg) return;
  const c = charts[svg.dataset.key];
  if (!c || c.pts.length < 2) return;
  const box = svg.getBoundingClientRect(), sx = (e.clientX - box.left) / box.width * c.W;
  let p = c.pts[0];
  for (const q of c.pts) if (c.x(q[0]) <= sx) p = q;
  const cross = svg.querySelector('.cross');
  cross.setAttribute('x1', c.x(p[0])); cross.setAttribute('x2', c.x(p[0])); cross.style.display = '';
  const tip = svg.parentNode.querySelector('.tip');
  tip.innerHTML = `<div style="color:var(--muted)">${c.week ? new Date(p[0]).toLocaleDateString('en-US', { weekday: 'short' }) + ' ' : ''}${hhmm(p[0])}</div>
    <div><i style="background:${c.col}"></i>${c.week ? 'Weekly' : '5-hour'} ${p[1]}%</div>`;
  tip.style.display = 'block';
  const host = svg.parentNode.getBoundingClientRect();
  tip.style.left = Math.min(e.clientX - host.left + 14, host.width - tip.offsetWidth - 6) + 'px';
  tip.style.top = (box.top - host.top + 4) + 'px';
});
$('#cards').addEventListener('mouseleave', () => {
  hovering = false;
  document.querySelectorAll('#cards .tip').forEach(t => (t.style.display = 'none'));
  document.querySelectorAll('#cards .cross').forEach(l => (l.style.display = 'none'));
});
