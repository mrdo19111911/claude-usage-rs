// Shared by the page's scripts (loaded first): helpers, the API call, formatting.
// Parts: $, H, globals S/tableView/clockSkew, api, hhmm, resetLabel, ago, color, esc,
// status, bar.

const $ = s => document.querySelector(s);
const H = 3600e3;
let S = null, tableView = false, clockSkew = 0;

const api = (p, opt = {}) =>
  fetch(p, { ...opt, headers: { 'Content-Type': 'application/json', 'X-CUD': '1' } })
    .then(async r => { const j = await r.json(); if (!r.ok) throw new Error(j.error); return j; });

function hhmm(t) { return new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false }); }
// Time left first ("in 19h15m"), then the clock time with its day.
function resetLabel(t) {
  if (!t) return '';
  const m = Math.max(0, Math.round((t - (Date.now() - clockSkew)) / 60000));
  const left = m >= 1440 ? `${Math.floor(m / 1440)}d ${Math.floor(m % 1440 / 60)}h` : m >= 60 ? `${Math.floor(m / 60)}h${String(m % 60).padStart(2, '0')}m` : `${m}m`;
  const day = new Date(t).toDateString() === new Date().toDateString() ? 'today' : new Date(t).toLocaleDateString('en-US', { weekday: 'short' });
  return `in ${left} (${day} ${hhmm(t)})`;
}
function ago(t) {
  const s = Math.max(0, Math.round((Date.now() - clockSkew - t) / 1000));
  if (s < 60) return s + ' s ago';
  if (s < 3600) return Math.round(s / 60) + ' min ago';
  return Math.round(s / 3600) + ' h ago';
}
function color(u) { return u >= 97 ? 'var(--red)' : u >= 70 ? 'var(--amber)' : 'var(--green)'; }
function esc(s) { return String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c])); }

function status(a) {
  const th = S.settings.threshold;
  if (a.error && !a.usage) return ['<span style="color:var(--red)">!</span>', 'Error'];
  if (!a.usage) return ['…', 'Waiting'];
  const { five, week } = a.usage;
  if (week.u >= 100) return ['<span style="color:var(--red)">⛔</span>', 'Weekly limit reached'];
  if (five.u >= 100) return ['<span style="color:var(--red)">⛔</span>', '5-hour limit reached'];
  if (a.id === S.activeId) return ['<span class="dot" style="margin:0"></span>', 'Active'];
  if (week.u >= 99) return ['<span style="color:#f08a5d">◕</span>', 'Weekly almost used up'];
  if (week.u >= 90) return ['<span style="color:var(--amber)">▲</span>', 'Weekly running low'];
  if (five.u >= th) return ['<span style="color:var(--amber)">▲</span>', '5-hour running low'];
  if (!a.enabled) return ['<span style="color:var(--dim)">○</span>', 'Off'];
  return ['<span class="dot" style="margin:0;background:var(--blue)"></span>', 'Ready'];
}

function bar(w, showReset = true) {
  const u = w.u;
  return `<div class="bar"><div class="track"><div class="fill" style="width:${Math.max(u, u > 0 ? 3 : 0)}%;background:${color(u)}"></div></div><div class="pct">${u}%</div></div>
    <div class="left"><b>${100 - u}% left</b>${!showReset ? '' : w.resetsAt ? ' · resets ' + resetLabel(w.resetsAt) : ' · not started yet'}</div>`;
}
