// "Usage over time": one small chart (or table) per account.
// Parts: rangeH (+ saved range), chartPoints, miniChart, hovering, renderChart, charts,
// #chart mousemove / mouseenter / mouseleave handlers, #range click handler.

// Small multiples: one chart per account, the same 0-100 scale and time range in all,
// so they compare at a glance. Two lines each: 5-hour and weekly share used.
let rangeH = 24;
try { rangeH = +localStorage.getItem('cud-range') || 24; } catch (e) {}

function chartPoints(a, start, now) {
  const pts = a.history.filter(p => p[0] >= start);
  const before = a.history.filter(p => p[0] < start).pop();
  if (before) pts.unshift([start, before[1], before[2]]);
  // Figures hold until the next check: carry the last one to now.
  if (pts.length && a.usage) pts.push([now, a.usage.five.u, a.usage.week.u]);
  return pts;
}

function miniChart(a, start, now) {
  const th = S.settings.threshold, end = now;
  const pts = chartPoints(a, start, now);
  const W = 400, Ht = 150, L = 30, R = 40, T = 10, B = 22;
  const x = t => L + (t - start) / (end - start) * (W - L - R);
  const y = u => T + (1 - u / 100) * (Ht - T - B);
  let g = '';
  for (const v of [0, 50, 100]) g += `<line x1="${L}" x2="${W - R}" y1="${y(v)}" y2="${y(v)}" stroke="var(--grid)"/><text x="${L - 6}" y="${y(v) + 4}" text-anchor="end">${v}</text>`;
  const ticks = rangeH === 168 ? 7 : 4;
  for (let i = 0; i <= ticks; i++) {
    const t = start + i * (end - start) / ticks;
    const lab = rangeH === 168 ? new Date(t).toLocaleDateString('en-US', { weekday: 'short' }) : hhmm(t);
    g += `<text x="${x(t)}" y="${Ht - 6}" text-anchor="${i === 0 ? 'start' : i === ticks ? 'end' : 'middle'}">${lab}</text>`;
  }
  g += `<line x1="${L}" x2="${W - R}" y1="${y(th)}" y2="${y(th)}" stroke="var(--muted)" stroke-width="1" stroke-dasharray="4 4"/>`;
  if (pts.length < 2) {
    g += `<text x="${(L + W - R) / 2}" y="${y(50)}" text-anchor="middle">${a.usage ? 'Not enough figures in this range yet' : 'No figures yet'}</text>`;
  } else {
    const line = i => pts.map((p, k) => `${k ? 'L' : 'M'}${x(p[0]).toFixed(1)},${y(p[i]).toFixed(1)}`).join('');
    g += `<path d="${line(2)}" fill="none" stroke="var(--s-week)" stroke-width="2" stroke-linejoin="round"/>`;
    g += `<path d="${line(1)}" fill="none" stroke="var(--s-five)" stroke-width="2" stroke-linejoin="round"/>`;
    // Direct labels at the line ends, nudged apart when they would overlap.
    const last = pts[pts.length - 1];
    let y5 = y(last[1]), yw = y(last[2]);
    if (Math.abs(y5 - yw) < 12) { const m = (y5 + yw) / 2; y5 = m + (last[1] >= last[2] ? -6 : 6); yw = m + (last[1] >= last[2] ? 6 : -6); }
    g += `<circle cx="${x(last[0])}" cy="${y(last[2])}" r="3.5" fill="var(--s-week)" stroke="var(--card)" stroke-width="2"/>`;
    g += `<circle cx="${x(last[0])}" cy="${y(last[1])}" r="3.5" fill="var(--s-five)" stroke="var(--card)" stroke-width="2"/>`;
    g += `<text x="${W - R + 6}" y="${y5 + 4}" style="fill:var(--text)">${last[1]}%</text>`;
    g += `<text x="${W - R + 6}" y="${yw + 4}" style="fill:var(--text)">${last[2]}%</text>`;
  }
  g += `<line class="cross" x1="0" x2="0" y1="${T}" y2="${Ht - B}" stroke="#777" style="display:none"/>`;
  g += `<rect x="${L}" y="0" width="${W - L - R}" height="${Ht}" fill="transparent"/>`;
  return { svg: `<svg viewBox="0 0 ${W} ${Ht}" data-id="${a.id}">${g}</svg>`, pts, x, W, L, R };
}

let hovering = false;
function renderChart(force) {
  // A redraw would drop the tooltip under the pointer: wait until it leaves.
  if (hovering && !force) return;
  const now = Date.now() - clockSkew, start = now - rangeH * H;
  $('#thLegend').textContent = S.settings.threshold;
  document.querySelectorAll('#range button').forEach(b => b.classList.toggle('on', +b.dataset.range === rangeH));
  if (!S.accounts.length) { $('#chart').innerHTML = '<p class="note">No accounts yet.</p>'; return; }
  const inDesk = uuid => S.desktop.accounts.some(d => d.signedIn && d.uuid === uuid);
  if (tableView) {
    $('#chart').innerHTML = S.accounts.map(a => {
      const pts = chartPoints(a, start, now).slice().reverse().slice(0, 40);
      return `<div class="mini"><h3>${esc(a.email)}</h3><table class="tableview"><thead><tr><th>Time</th><th>5-hour</th><th>Weekly</th></tr></thead><tbody>${
        pts.map(p => `<tr><td>${rangeH === 168 ? new Date(p[0]).toLocaleDateString('en-US', { weekday: 'short' }) + ' ' : ''}${hhmm(p[0])}</td><td>${p[1]}%</td><td>${p[2]}%</td></tr>`).join('')}</tbody></table></div>`;
    }).join('');
    return;
  }
  charts = {};
  $('#chart').innerHTML = S.accounts.map(a => {
    const c = miniChart(a, start, now);
    charts[a.id] = c;
    const where = whereTags(a.id === S.activeId, inDesk(a.uuid));
    const u = a.usage;
    const sub = u ? (u.week.u >= 100 ? 'Weekly limit reached · resets ' + resetLabel(u.week.resetsAt)
      : u.five.u >= 100 ? '5-hour limit reached · resets ' + resetLabel(u.five.resetsAt)
      : u.five.resetsAt ? '5-hour resets ' + resetLabel(u.five.resetsAt) : '5-hour window not started (no use yet)') : 'Waiting for figures';
    return `<div class="mini ${where ? 'inuse' : ''}">
      <h3>${where ? '<span class="dot"></span>' : ''}${esc(a.email)}</h3>
      <div class="meta">${where}${sub}</div>
      ${c.svg}<div class="tip"></div></div>`;
  }).join('');
}

let charts = {};
// Crosshair and tooltip: the figures in force at the pointer's time.
$('#chart').addEventListener('mousemove', e => {
  const svg = e.target.closest('svg[data-id]');
  document.querySelectorAll('.mini .tip').forEach(t => { if (!svg || t.parentNode !== svg.parentNode) t.style.display = 'none'; });
  document.querySelectorAll('.mini .cross').forEach(l => { if (!svg || l.ownerSVGElement !== svg) l.style.display = 'none'; });
  if (!svg) return;
  const c = charts[svg.dataset.id];
  if (!c || c.pts.length < 2) return;
  const box = svg.getBoundingClientRect(), sx = (e.clientX - box.left) / box.width * c.W;
  let p = c.pts[0];
  for (const q of c.pts) if (c.x(q[0]) <= sx) p = q;
  const cross = svg.querySelector('.cross');
  cross.setAttribute('x1', c.x(p[0])); cross.setAttribute('x2', c.x(p[0])); cross.style.display = '';
  const tip = svg.parentNode.querySelector('.tip');
  tip.innerHTML = `<div style="color:var(--muted)">${rangeH === 168 ? new Date(p[0]).toLocaleDateString('en-US', { weekday: 'short' }) + ' ' : ''}${hhmm(p[0])}</div>
    <div><i style="background:var(--s-five)"></i>5-hour ${p[1]}%</div><div><i style="background:var(--s-week)"></i>Weekly ${p[2]}%</div>`;
  tip.style.display = 'block';
  const host = svg.parentNode.getBoundingClientRect();
  const left = e.clientX - host.left + 14;
  tip.style.left = Math.min(left, host.width - tip.offsetWidth - 6) + 'px';
  tip.style.top = (svg.getBoundingClientRect().top - host.top + 4) + 'px';
});
$('#chart').addEventListener('mouseenter', () => (hovering = true));
$('#chart').addEventListener('mouseleave', () => {
  hovering = false;
  document.querySelectorAll('.mini .tip').forEach(t => (t.style.display = 'none'));
  document.querySelectorAll('.mini .cross').forEach(l => (l.style.display = 'none'));
});
$('#range').addEventListener('click', e => {
  const b = e.target.closest('[data-range]');
  if (!b) return;
  rangeH = +b.dataset.range;
  try { localStorage.setItem('cud-range', rangeH); } catch (err) {}
  renderChart(true);
});
