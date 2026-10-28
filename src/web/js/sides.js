// The "In use now" panels: Claude CLI and Claude Desktop.
// Parts: whereTags, usageHint, side, renderSides.

// Where an account is in use, one colored tag per login.
function whereTags(code, desk) {
  return (code ? '<span class="tag code sm">Claude CLI</span>' : '') + (desk ? '<span class="tag desk sm">Claude Desktop</span>' : '');
}

function usageHint(u) { return u ? ` · 5h ${u.five.u}% · week ${u.week.u}%` : ''; }

// One of the two "In use now" panels; `opts` are [action, label] for its switch menu.
function side(kind, title, sub, email, u, note, opts, busy) {
  return `<div class="side-head"><span class="tag ${kind}">${title}</span><span class="note">${sub}</span></div>
    <div class="email" style="margin-top:10px">${email ? '<span class="dot"></span>' + esc(email) : '<span class="note">Not signed in</span>'}</div>
    ${u ? usageBars(u) : ''}
    <div class="note" style="margin-top:8px">${note}</div>
    ${opts.length ? `<div class="side-switch"><select aria-label="Switch ${title} to">${opts.map(([v, l]) => `<option value="${esc(v)}">${esc(l)}</option>`).join('')}</select><button class="btn" data-side ${busy ? 'disabled' : ''}>Switch</button></div>` : ''}`;
}

function renderSides() {
  // A redraw would close the menu the user is choosing from.
  if ($('#sides').contains(document.activeElement)) return;
  const D = S.desktop, s = S.settings;
  const cur = S.accounts.find(a => a.id === S.activeId);
  const codeOpts = S.accounts.filter(a => a !== cur).map(a => [`activate:${a.id}`, a.email + usageHint(a.usage) + (a.enabled ? '' : ' · off')]);
  $('#sideCode').innerHTML = side('code', 'Claude CLI', 'Terminal and VS Code: one shared login', cur && cur.email, cur && cur.usage,
    s.autoSwitch ? `Auto-switch moves it to another account at ${s.threshold}%.` : 'Auto-switch is off.', codeOpts, false);
  if (!D.found || S.demo) {
    $('#sideDesk').innerHTML = side('desk', 'Claude Desktop', 'Its own sign-in', null, null, 'Claude Desktop was not found on this computer.', [], true);
    return;
  }
  const dcur = D.accounts.find(d => d.signedIn);
  const deskOpts = accountRows().filter(r => !(r.d && r.d.signedIn)).map(({ a, d }) => {
    const email = (a && a.email) || (d && (d.email || d.label)) || '';
    return d && d.hasSignIn ? [`dswitch:${d.uuid}`, email + usageHint(a ? a.usage : d.usage)] : [`dfirst:${email}`, email + ' · first time: sign in once'];
  });
  $('#sideDesk').innerHTML = side('desk', 'Claude Desktop', 'Its own sign-in: auto-switch never changes it', dcur && (dcur.email || dcur.label), dcur && dcur.usage,
    D.switching ? 'Closing and opening again on the new account…' : 'Switching closes the app and opens it again (running tasks stop).', deskOpts, D.switching);
}
