// The "Claude accounts" section: its accounts, shared parts, and drawing it.
// Parts: accountRows, checkLine, chips, usageBars, renderRows. The cards: card.js.

// One entry per account: the terminal logins, plus Claude Desktop accounts with none.
function accountRows() {
  const D = S.desktop;
  const rows = S.accounts.map(a => ({ a, d: D.accounts.find(d => a.uuid && d.uuid === a.uuid) }));
  for (const d of D.accounts) if (!rows.some(r => r.d === d)) rows.push({ a: null, d });
  // An account at 100% of a limit goes last, after those with only a Desktop sign-in.
  const full = r => !!(r.a && r.a.usage && [r.a.usage.five, r.a.usage.week].some(w => effWin(w).u >= 100));
  return rows.filter(r => !full(r)).concat(rows.filter(full));
}

// When the figures are from, and when Anthropic is asked next (sooner near a limit,
// not at all while a limit is full).
function checkLine(a) {
  const t = Date.now() - clockSkew, parts = [];
  if (a.usage) parts.push(`checked <span data-ago="${a.usage.fetchedAt}">${ago(a.usage.fetchedAt)}</span>`);
  if (a.nextCheck > t + 5e3) parts.push('next check ' + resetLabel(a.nextCheck));
  return parts.length ? `<div class="meta">${parts.join(' · ')}</div>` : '';
}

// A small chip per model with a weekly limit of its own ("Fable 0%").
function chips(u) {
  return (u.scoped || []).map(s => `<span class="scoped" style="--c:${color(s.u)}" title="${esc(s.name)} weekly limit${s.resetsAt ? ' · resets ' + resetLabel(s.resetsAt) : ''}">${esc(s.name)} <b>${s.u}%</b></span>`).join('');
}

// The 5-hour and weekly bars side by side (the "In use now" panels).
function usageBars(u) {
  return `<div class="side-bars"><div><div class="note">5-hour</div>${bar(u.five)}</div><div><div class="note">Weekly</div>${bar(u.week)}${chips(u)}</div></div>`;
}

function renderRows() {
  const th = S.settings.threshold, thW = S.settings.weekThreshold ?? 99;
  renderCards();

  const on = S.accounts.filter(a => a.enabled && a.usage);
  const ready = on.filter(a => a.usage.five.u < th && a.usage.week.u < thW).length;
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
