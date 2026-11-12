// The "Claude accounts" section: one card per account (docs/specs/account-cards.md).
// Parts: login, untilLine, cliLogin, deskLogin, who, paneLeft, pane, tablePanes, card, cardPaneWidth,
// renderCards.

// One login of the account, drawn the same way for Claude Code and Claude Desktop (equal
// peers): its name and "✓ In use" or the button that uses it there, then under them its
// auto-switch switch (or why there is none). `o`: { on, act: [attr, value], label, busy,
// auto: [attr, value, checked, title], noAuto }.
function login(kind, name, o) {
  const btn = o.on ? '<span class="on">✓ In use</span>'
    : o.act ? `<button class="btn" ${o.act[0]}="${esc(o.act[1])}" ${o.busy ? 'disabled' : ''}>${o.label}</button>`
    : `<span class="note">${o.label}</span>`;
  const auto = o.auto
    ? `<label class="auto" title="${o.auto[3]}"><span class="switch"><input type="checkbox" ${o.auto[0]}="${esc(o.auto[1])}" ${o.auto[2] ? 'checked' : ''}><span></span></span>allow auto-switch</label>`
    : `<div class="auto note">${o.noAuto || ''}</div>`;
  return `<div class="login ${kind}${o.on ? ' here' : ''}"><div class="login-top"><span class="name">${name}</span>${btn}</div>${auto}${o.until || ''}</div>`;
}

// Until when a saved Claude Desktop sign-in lasts (docs/specs/desktop-signed-out.md):
// claude.ai keeps it 28 days from its last use; amber from 7 days left.
function untilLine(ms) {
  if (!ms) return '';
  const days = Math.max(0, Math.floor((ms - (Date.now() - clockSkew)) / 864e5));
  const date = new Date(ms).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  const tip = 'Each use renews this saved sign-in for 28 days; unused, it ends then. Due within 7 days, it is renewed at 4:00 (Settings). Log out in Claude ends it at once.';
  return `<div class="until note${days < 7 ? ' soon' : ''}" title="${tip}">Saved sign-in lasts until ${date} (${days} day${days === 1 ? '' : 's'})</div>`;
}

// Claude Code (terminal and VS Code): "Switch to this" switches at once; not signed in yet,
// "Sign in" adds one.
function cliLogin(a, d) {
  if (!a) return { act: ['data-signin', (d && d.email) || ''], label: 'Sign in', noAuto: 'not signed in yet' };
  return { on: a.id === S.activeId, act: ['data-activate', a.id], label: 'Switch to this',
    auto: ['data-toggle', a.id, a.enabled, 'Auto-switch may move Claude Code to this account'] };
}

// Claude Desktop: "Switch to this" restarts it on this account; the first time,
// it opens signed out for a sign-in. Its switch only once a sign-in is saved.
function deskLogin(a, d) {
  const D = S.desktop, busy = D.switching;
  if (S.demo) return { label: 'Not in the demo' };
  if (!D.found) return { label: 'Claude Desktop not installed' };
  if (!d || !d.hasSignIn) return { on: d && d.signedIn, act: ['data-dfirst', (a && a.email) || (d && d.email) || ''],
    label: 'Sign in here', busy, noAuto: 'available after first sign-in' };
  // Its saved session has ended: opening it would land on the sign-in screen anyway.
  if (d.signInExpired) return { act: ['data-dfirst', (a && a.email) || d.email || ''], label: 'Sign in again', busy,
    noAuto: 'saved sign-in expired: skipped by auto-switch' };
  return { on: d.signedIn, act: ['data-dswitch', d.uuid], label: 'Switch to this', busy, until: untilLine(d.signInUntil),
    auto: ['data-dauto', d.uuid, d.auto, 'Auto-switch may move Claude Desktop to this account when the one in use reaches a limit'] };
}

// The account: on the left email (dot when in use anywhere), ✕, its problem in plain words
// (errBadge, tidy.js); on the right
// plan and state, checks.
// ✕ removes the Claude Code login and the saved Desktop sign-in, each where not in use.
function who(a, d) {
  const email = (a && a.email) || (d && (d.email || d.label)) || '';
  const inUse = (a && a.id === S.activeId) || (d && d.signedIn);
  const rmCli = a && a.id !== S.activeId, rmDesk = d && !d.signedIn && !S.demo;
  const rm = rmCli || rmDesk ? `<button class="btn x" ${rmCli ? `data-remove="${a.id}"` : ''} ${rmDesk ? `data-dremove="${d.uuid}"` : ''} title="Remove from this app">✕</button>` : '';
  const [icon, label] = a ? status(a) : ['', ''];
  return `<div class="who"><div class="who-l"><div class="email ${nameTone(a)}">${inUse ? '<span class="dot"></span>' : ''}${esc(email)}${rm}</div>
    ${errBadge(a)}</div>
    <div class="who-r"><div class="meta">${a ? `${esc(a.plan)} · ${icon} ${label}` : 'Not signed in to Claude Code yet: sign in to see its usage'}</div>${a ? checkLine(a) : ''}${a ? `<button class="btn check" data-check="${a.id}" title="Check this account's usage right now">Check now</button>` : ''}</div></div>`;
}

// What is left of a window, for its title row: "57% left · in 1d 14h" (the clock time is
// under the chart's axis), in the bar colours.
function paneLeft(w) {
  return `<span class="pane-l"><b style="color:${color(w.u)}">${100 - w.u}% left</b> · ${w.resetsAt ? 'in ' + timeLeft(w.resetsAt) : 'no use yet'}</span>`;
}

// One window: its title row (the name, weekly with the model chips, and what is left),
// then its chart over the window; `W` as winChart's. No bar: the chart's dot and its reset
// line say the same (docs/specs/account-cards.md, "Less space").
function pane(a, k, now, W) {
  const c = winChart(a, k, now, W);
  charts[`${a.id}:${k}`] = c;
  const w = effWin(k === 1 ? a.usage.five : a.usage.week);
  return `<div class="pane"><div class="pane-t"><span class="pane-n">${k === 1 ? '5-hour' : 'Weekly'}${k === 2 ? chips(a.usage) : ''}</span>${paneLeft(w)}</div>${c.svg}<div class="tip"></div></div>`;
}

// Table view: the figures of the weekly window, newest first.
function tablePanes(a, now) {
  const [start] = winSpan(a.usage.week, 7 * 24 * H, now);
  const pts = a.history.filter(p => p[0] >= start).concat([[now, a.usage.five.u, a.usage.week.u]]).reverse().slice(0, 40);
  const day = t => new Date(t).toLocaleDateString('en-US', { weekday: 'short' });
  return `<div class="scroll"><table class="tableview"><thead><tr><th>Time</th><th>5-hour</th><th>Weekly</th></tr></thead><tbody>${
    pts.map(p => `<tr><td>${day(p[0])} ${hhmm(p[0])}</td><td>${p[1]}%</td><td>${p[2]}%</td></tr>`).join('')}</tbody></table></div>`;
}

function card(a, d, now, W) {
  const inUse = (a && a.id === S.activeId) || (d && d.signedIn);
  const body = a && a.usage
    ? (tableView ? tablePanes(a, now) : `<div class="panes">${pane(a, 1, now, W)}${pane(a, 2, now, W)}</div>`)
    : a ? '<div class="note">Usage not checked yet</div>' : '';
  return `<div class="acard${inUse ? ' inuse' : ''}${a ? '' : ' muted'}">${who(a, d)}
    <div class="logins">${login('code', 'Claude Code', cliLogin(a, d))}${login('desk', 'Claude Desktop', deskLogin(a, d))}</div>
    ${body}</div>`;
}

// A card pane's width in pixels: the grid's column less the card's padding and border,
// halved less the gap when the panes sit side by side (docs/specs/account-cards.md, "Less
// space" 6). The charts are drawn that wide, so they stay 104 px tall.
function cardPaneWidth() {
  const inner = (parseFloat(getComputedStyle($('#cards')).gridTemplateColumns) || 0) - 30;
  return Math.round(matchMedia('(max-width: 760px)').matches ? inner : (inner - 14) / 2);
}

function renderCards(force) {
  if (hovering && !force) return;
  const now = Date.now() - clockSkew, W = Math.max(300, cardPaneWidth());
  charts = {};
  $('#cards').innerHTML = accountRows().map(({ a, d }) => card(a, d, now, W)).join('')
    || '<p class="note">No accounts yet. Click “+ Add account” and sign in.</p>';
}
