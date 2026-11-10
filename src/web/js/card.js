// The "Claude accounts" section: one card per account (docs/specs/account-cards.md).
// Parts: login, cliLogin, deskLogin, who, paneLeft, pane, tablePanes, card, renderCards.

// One login of the account, drawn the same way for Claude CLI and Claude Desktop (equal
// peers): its name and "✓ In use" or the button that uses it there, then under them its
// auto-switch switch (or why there is none). `o`: { on, act: [attr, value], label, busy,
// auto: [attr, value, checked, title], noAuto }.
function login(kind, name, o) {
  const btn = o.on ? '<span class="on">✓ In use</span>'
    : o.act ? `<button class="btn" ${o.act[0]}="${esc(o.act[1])}" ${o.busy ? 'disabled' : ''}>${o.label}</button>`
    : `<span class="note">${o.label}</span>`;
  const auto = o.auto
    ? `<label class="auto" title="${o.auto[3]}"><span class="switch"><input type="checkbox" ${o.auto[0]}="${esc(o.auto[1])}" ${o.auto[2] ? 'checked' : ''}><span></span></span>auto-switch</label>`
    : `<div class="auto note">${o.noAuto || ''}</div>`;
  return `<div class="login ${kind}${o.on ? ' here' : ''}"><div class="login-top"><span class="name">${name}</span>${btn}</div>${auto}</div>`;
}

// Claude CLI (terminal and VS Code): "Use here" switches at once; with no login yet,
// "Sign in" adds one.
function cliLogin(a, d) {
  if (!a) return { act: ['data-signin', (d && d.email) || ''], label: 'Sign in', noAuto: 'no login yet' };
  return { on: a.id === S.activeId, act: ['data-activate', a.id], label: 'Use here',
    auto: ['data-toggle', a.id, a.enabled, 'Auto-switch may move Claude CLI to this account'] };
}

// Claude Desktop: "Use here" closes and opens it again on this account; the first time,
// it opens signed out for a sign-in. Its switch only once a sign-in is saved.
function deskLogin(a, d) {
  const D = S.desktop, busy = D.switching;
  if (S.demo) return { label: 'Not in the demo' };
  if (!D.found) return { label: 'Not on this computer' };
  if (!d || !d.hasSignIn) return { on: d && d.signedIn, act: ['data-dfirst', (a && a.email) || (d && d.email) || ''],
    label: 'Use here (first time)', busy, noAuto: 'after its first sign-in here' };
  return { on: d.signedIn, act: ['data-dswitch', d.uuid], label: 'Use here', busy,
    auto: ['data-dauto', d.uuid, d.auto, 'Auto-switch may move Claude Desktop to this account when the one in use reaches a limit'] };
}

// The account: on the left email (dot when in use anywhere), ✕, error; on the right
// plan and state, checks.
// ✕ removes the Claude CLI login and the saved Desktop sign-in, each where not in use.
function who(a, d) {
  const email = (a && a.email) || (d && (d.email || d.label)) || '';
  const inUse = (a && a.id === S.activeId) || (d && d.signedIn);
  const rmCli = a && a.id !== S.activeId, rmDesk = d && !d.signedIn && !S.demo;
  const rm = rmCli || rmDesk ? `<button class="btn x" ${rmCli ? `data-remove="${a.id}"` : ''} ${rmDesk ? `data-dremove="${d.uuid}"` : ''} title="Remove">✕</button>` : '';
  const [icon, label] = a ? status(a) : ['', ''];
  return `<div class="who"><div class="who-l"><div class="email">${inUse ? '<span class="dot"></span>' : ''}${esc(email)}${rm}</div>
    ${a && a.error ? `<div class="err">${esc(a.error)}</div>` : ''}</div>
    <div class="who-r"><div class="meta">${a ? `${esc(a.plan)} · ${icon} ${label}` : 'No Claude CLI login yet: sign in to see its figures'}</div>${a ? checkLine(a) : ''}</div></div>`;
}

// What is left of a window, for its title row: "57% left · in 1d 14h" (the clock time is
// under the chart's axis), in the bar colours.
function paneLeft(w) {
  return `<span class="pane-l"><b style="color:${color(w.u)}">${100 - w.u}% left</b> · ${w.resetsAt ? 'in ' + timeLeft(w.resetsAt) : 'not started'}</span>`;
}

// One window: its title row with what is left, its chart over the window (and, weekly, the
// model chips). No bar: the chart's dot and its reset line say the same (docs/specs/
// account-cards.md, "Less space").
function pane(a, k, now) {
  const c = winChart(a, k, now);
  charts[`${a.id}:${k}`] = c;
  const w = effWin(k === 1 ? a.usage.five : a.usage.week);
  return `<div class="pane"><div class="pane-t">${k === 1 ? '5-hour' : 'Weekly'}${paneLeft(w)}</div>${c.svg}<div class="tip"></div>${k === 2 ? chips(a.usage) : ''}</div>`;
}

// Table view: the figures of the weekly window, newest first.
function tablePanes(a, now) {
  const [start] = winSpan(a.usage.week, 7 * 24 * H, now);
  const pts = a.history.filter(p => p[0] >= start).concat([[now, a.usage.five.u, a.usage.week.u]]).reverse().slice(0, 40);
  const day = t => new Date(t).toLocaleDateString('en-US', { weekday: 'short' });
  return `<div class="scroll"><table class="tableview"><thead><tr><th>Time</th><th>5-hour</th><th>Weekly</th></tr></thead><tbody>${
    pts.map(p => `<tr><td>${day(p[0])} ${hhmm(p[0])}</td><td>${p[1]}%</td><td>${p[2]}%</td></tr>`).join('')}</tbody></table></div>`;
}

function card(a, d, now) {
  const inUse = (a && a.id === S.activeId) || (d && d.signedIn);
  const body = a && a.usage
    ? (tableView ? tablePanes(a, now) : `<div class="panes">${pane(a, 1, now)}${pane(a, 2, now)}</div>`)
    : a ? '<div class="note">Waiting for figures</div>' : '';
  return `<div class="acard${inUse ? ' inuse' : ''}${a ? '' : ' muted'}">${who(a, d)}
    <div class="logins">${login('code', 'Claude CLI', cliLogin(a, d))}${login('desk', 'Claude Desktop', deskLogin(a, d))}</div>
    ${body}</div>`;
}

function renderCards(force) {
  if (hovering && !force) return;
  const now = Date.now() - clockSkew;
  charts = {};
  $('#cards').innerHTML = accountRows().map(({ a, d }) => card(a, d, now)).join('')
    || '<p class="note">No accounts yet. Click “+ Add account” and sign in.</p>';
}
