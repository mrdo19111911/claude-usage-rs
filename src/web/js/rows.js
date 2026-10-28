// The "Claude accounts" table.
// Parts: accountRows, checkLine, useCells, renderRows.

// One row per account: the terminal logins, plus Claude Desktop accounts with none.
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
  return parts.length ? `<div class="meta" style="margin-top:6px">${parts.join(' · ')}</div>` : '';
}

// The table's last two cells: one per login, each switched on its own.
function useCells(a, d) {
  const D = S.desktop, busy = D.switching;
  const term = a && a.id === S.activeId
    ? '<div class="on">✓ In use</div>'
    : a ? `<button class="btn" data-activate="${a.id}">Use here</button>`
    : `<button class="btn" data-signin="${esc(d.email || '')}">Sign in</button>`;
  if (!D.found || S.demo) return `<td><div class="use">${term}</div></td><td><span class="note">—</span></td>`;
  const email = (a && a.email) || (d && d.email) || '';
  const desk = d && d.signedIn ? '<div class="on">✓ In use</div>'
    : d && d.hasSignIn ? `<button class="btn" data-dswitch="${d.uuid}" ${busy ? 'disabled' : ''}>Use here</button>`
    : `<button class="btn" data-dfirst="${esc(email)}" ${busy ? 'disabled' : ''}>Use here (first time)</button>`;
  return `<td><div class="use">${term}</div></td><td><div class="use">${desk}</div></td>`;
}

function renderRows() {
  const th = S.settings.threshold;
  $('#rows').innerHTML = accountRows().map(({ a, d }) => {
    const active = a && a.id === S.activeId;
    const inDesk = d && d.signedIn;
    const u = a ? a.usage : d.usage;
    const [icon, label] = a ? status(a) : ['<span style="color:var(--dim)">○</span>', 'Claude Desktop only'];
    const email = a ? a.email : (d.email || d.label);
    const removable = a ? !active : !inDesk;
    return `<tr class="${active || inDesk ? 'active' : ''}">
      <td><div class="acct">
        ${a ? `<label class="switch" title="Auto-switch may use it"><input type="checkbox" data-toggle="${a.id}" ${a.enabled ? 'checked' : ''}><span></span></label>` : ''}
        <div>
          <div class="email">${active || inDesk ? '<span class="dot"></span>' : ''}${esc(email)}</div>
          <div class="meta">${whereTags(active, inDesk)}${a ? esc(a.plan) : 'no Claude CLI login yet'}</div>
          <div class="row-actions">${removable ? `<button class="btn" ${a ? `data-remove="${a.id}"` : `data-dremove="${d.uuid}"`} title="Remove">✕</button>` : ''}</div>
        </div></div></td>
      <td><span class="pill">${icon} ${label}</span>${a && a.error ? `<div class="err">${esc(a.error)}</div>` : ''}${a ? checkLine(a) : ''}</td>
      <td style="min-width:200px">${u ? bar(u.five) : '<span class="note">—</span>'}</td>
      <td style="min-width:220px">${u ? bar(u.week) : '<span class="note">—</span>'}</td>
      ${useCells(a, d)}
    </tr>`;
  }).join('') || `<tr><td colspan="6" class="note">No accounts yet. Click “+ Add account” above and sign in.</td></tr>`;

  const on = S.accounts.filter(a => a.enabled && a.usage);
  const ready = on.filter(a => a.usage.five.u < th && a.usage.week.u < th).length;
  const head = on.reduce((s, a) => s + (100 - a.usage.week.u), 0);
  $('#summary').innerHTML = S.loggingIn && loginUrl
    ? `Sign-in page opened in your browser: sign in with the account to add. Not there? <a href="${esc(loginUrl)}" target="_blank" rel="noopener">Open it</a>.`
    : esc(`${ready} ready now · weekly headroom ${head}% (≈ ${(head / 100).toFixed(1)} accounts)`);
  $('#cliAdd').disabled = S.demo;

  const D = S.desktop, box = $('#desktopSub');
  const msg = D.switching ? 'Claude Desktop is closing and will open again (up to a minute)…'
    : D.expect ? `Claude Desktop is open and signed out: click <b>Continue with Google</b> there and choose <b>${esc(D.expect)}</b>.`
    : '';
  box.className = msg ? 'wait' : 'sub';
  box.style.display = msg ? '' : 'none';
  box.innerHTML = msg;
}
