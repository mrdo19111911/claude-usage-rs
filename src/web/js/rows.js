// The "Claude accounts" section: shared parts of its two lists, and drawing it.
// Parts: accountRows, checkLine, usageBars, listRow, renderRows.
// The lists themselves: list_cli.js (renderCli), list_desk.js (renderDesk).

// One entry per account: the terminal logins, plus Claude Desktop accounts with none.
function accountRows() {
  const D = S.desktop;
  const rows = S.accounts.map(a => ({ a, d: D.accounts.find(d => a.uuid && d.uuid === a.uuid) }));
  for (const d of D.accounts) if (!rows.some(r => r.d === d)) rows.push({ a: null, d });
  return rows;
}

// When the figures are from, and when Anthropic is asked next (sooner near a limit,
// not at all while a limit is full).
function checkLine(a) {
  const t = Date.now() - clockSkew, parts = [];
  if (a.usage) parts.push(`checked <span data-ago="${a.usage.fetchedAt}">${ago(a.usage.fetchedAt)}</span>`);
  if (a.nextCheck > t + 5e3) parts.push('next check ' + resetLabel(a.nextCheck));
  return parts.length ? `<div class="meta">${parts.join(' · ')}</div>` : '';
}

// The 5-hour and weekly bars side by side (also the "In use now" panels'), then a small
// chip per model with a weekly limit of its own ("Fable 0%").
function usageBars(u) {
  const chips = (u.scoped || []).map(s => `<span class="scoped" style="--c:${color(s.u)}" title="${esc(s.name)} weekly limit${s.resetsAt ? ' · resets ' + resetLabel(s.resetsAt) : ''}">${esc(s.name)} <b>${s.u}%</b></span>`).join('');
  return `<div class="side-bars"><div><div class="note">5-hour</div>${bar(u.five)}</div><div><div class="note">Weekly</div>${bar(u.week)}${chips}</div></div>`;
}

// One row of either list: `cls` is "inuse", "muted" or ""; `left` the account, `right`
// its buttons, `u` its usage (bars under both, or none).
function listRow(cls, left, right, u) {
  return `<div class="lrow ${cls}"><div class="lacct">${left}</div><div class="use">${right}</div>${u ? usageBars(u) : ''}</div>`;
}

function renderRows() {
  const th = S.settings.threshold;
  $('#cliRows').innerHTML = renderCli();
  $('#deskRows').innerHTML = renderDesk();

  const on = S.accounts.filter(a => a.enabled && a.usage);
  const ready = on.filter(a => a.usage.five.u < th && a.usage.week.u < th).length;
  const head = on.reduce((s, a) => s + (100 - a.usage.week.u), 0);
  $('#summary').innerHTML = S.loggingIn && loginUrl
    ? `Sign-in page opened in your browser: sign in with the account to add. Not there? <a href="${esc(loginUrl)}" target="_blank" rel="noopener">Open it</a>.`
    : esc(`${ready} ready now · weekly headroom ${head}% (≈ ${(head / 100).toFixed(1)} accounts)`);
  $('#cliAdd').disabled = S.demo;

  const D = S.desktop, box = $('#desktopSub');
  $('#deskAdd').disabled = S.demo || !D.found || D.switching;
  const msg = D.switching ? 'Claude Desktop is closing and will open again (up to a minute)…'
    : D.expect ? `Claude Desktop is open and signed out: click <b>Continue with Google</b> there and choose <b>${esc(D.expect)}</b>.`
    : '';
  box.className = msg ? 'wait' : 'sub';
  box.style.display = msg ? '' : 'none';
  box.innerHTML = msg;
}
