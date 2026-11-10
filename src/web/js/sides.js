// The "In use now" panels: Claude CLI and Claude Desktop, or one panel when both use one
// account (docs/specs/account-cards.md, "Less space").
// Parts: whereTags, usageHint, paneWidth, sideBody, emailLine, control, side, both, renderSides,
// the window's resize handler.

// Where an account is in use, one colored tag per login.
function whereTags(code, desk) {
  return (code ? '<span class="tag code sm">Claude CLI</span>' : '') + (desk ? '<span class="tag desk sm">Claude Desktop</span>' : '');
}

function usageHint(u) { return u ? ` · 5h ${u.five.u}% · week ${u.week.u}%` : ''; }

// A pane's width in pixels: the panel's inside, halved when the two panes sit side by side.
function paneWidth() {
  const inner = $('#sideCode').clientWidth - 32;
  return Math.round(matchMedia('(max-width: 760px)').matches ? inner : (inner - 14) / 2);
}

// Its account's two charts, as on its card (docs/specs/account-cards.md, "Charts in In
// use now"), drawn at the pane's width so they stay low ("Less space" 4); the bars only
// with no history here (a Desktop-only account), no figures yet, or in table view.
function sideBody(a, u, now) {
  const W = Math.max(300, paneWidth());
  if (a && a.usage && !tableView) return `<div class="panes">${pane(a, 1, now, W)}${pane(a, 2, now, W)}</div>`;
  return u ? usageBars(u) : '';
}

function emailLine(email) {
  return `<div class="email" style="margin-top:10px">${email ? '<span class="dot"></span>' + esc(email) : '<span class="note">Not signed in</span>'}</div>`;
}

// One app's note, one short line with the full text in its tooltip (`note` is [short,
// full]), and its switch menu (`opts` are [action, label]). `kind` puts the app's tag
// before the menu, for the shared panel.
function control(kind, title, note, opts, busy) {
  return `<div class="ctl"><div class="note" title="${esc(note[1])}">${note[0]}</div>
    ${opts.length ? `<div class="side-switch">${kind ? `<span class="tag ${kind} sm">${title}</span>` : ''}<select aria-label="Switch ${title} to">${opts.map(([v, l]) => `<option value="${esc(v)}">${esc(l)}</option>`).join('')}</select><button class="btn" data-side ${busy ? 'disabled' : ''}>Switch</button></div>` : ''}</div>`;
}

// One of the two "In use now" panels; `body` is sideBody's.
function side(kind, title, sub, email, body, note, opts, busy) {
  return `<div class="side-head"><span class="tag ${kind}">${title}</span><span class="note">${sub}</span></div>
    ${emailLine(email)}${body}${control(null, title, note, opts, busy)}`;
}

// Both apps on one account: one panel, the account's charts once, then each app's note
// and menu side by side. `code`, `desk`: [note, opts, busy].
function both(email, body, code, desk) {
  return `<div class="side-head"><span class="tag code">Claude CLI</span><span class="tag desk">Claude Desktop</span><span class="note">Both on this account</span></div>
    ${emailLine(email)}${body}<div class="ctls">${control('code', 'Claude CLI', ...code)}${control('desk', 'Claude Desktop', ...desk)}</div>`;
}

function renderSides() {
  // A redraw would close the menu the user is choosing from, or the tooltip under the pointer.
  if (hovering || $('#sides').contains(document.activeElement)) return;
  const D = S.desktop, s = S.settings, now = Date.now() - clockSkew;
  const cur = S.accounts.find(a => a.id === S.activeId);
  const codeOpts = S.accounts.filter(a => a !== cur).map(a => [`activate:${a.id}`, a.email + usageHint(a.usage) + (a.enabled ? '' : ' · off')]);
  const code = [s.autoSwitch ? [`Auto-switch at ${s.threshold}% 5-hour · ${s.weekThreshold ?? 99}% weekly`, `Auto-switch moves it to another account at ${s.threshold}% (5-hour) or ${s.weekThreshold ?? 99}% (weekly).`]
    : ['Auto-switch is off', 'Auto-switch is off: Claude CLI stays on its account.'], codeOpts, false];
  const found = D.found && !S.demo;
  const dcur = found && D.accounts.find(d => d.signedIn);
  const da = dcur && S.accounts.find(a => a.uuid && a.uuid === dcur.uuid);
  const same = !!(cur && da && cur === da);
  $('#sides').classList.toggle('both', same);
  const codeSide = () => side('code', 'Claude CLI', 'Terminal and VS Code: one shared login', cur && cur.email, sideBody(cur, cur && cur.usage, now), ...code);
  if (!found) {
    const nf = 'Claude Desktop was not found on this computer.';
    $('#sideCode').innerHTML = codeSide();
    $('#sideDesk').innerHTML = side('desk', 'Claude Desktop', 'Its own sign-in', null, '', [nf, nf], [], true);
    return;
  }
  const deskOpts = accountRows().filter(r => !(r.d && r.d.signedIn)).map(({ a, d }) => {
    const email = (a && a.email) || (d && (d.email || d.label)) || '';
    return d && d.hasSignIn ? [`dswitch:${d.uuid}`, email + usageHint(a ? a.usage : d.usage)] : [`dfirst:${email}`, email + ' · first time: sign in once'];
  });
  // Its auto-switch point is desktop_auto.rs DESKTOP_AT.
  const restart = 'Switching closes the app and opens it again (running tasks stop).', moving = 'Closing and opening again on the new account…';
  const dnote = D.switching ? [moving, moving]
    : s.autoSwitch ? ['Auto-switch at 98% · switching restarts it', 'Auto-switch moves it at 98% (5-hour or weekly), to accounts whose Desktop auto-switch is on. ' + restart]
    : ['Auto-switch is off · switching restarts it', 'Auto-switch is off. ' + restart];
  const desk = [dnote, deskOpts, D.switching];
  if (same) {
    $('#sideCode').innerHTML = both(cur.email, sideBody(cur, cur.usage, now), code, desk);
    $('#sideDesk').innerHTML = '';
    return;
  }
  $('#sideCode').innerHTML = codeSide();
  $('#sideDesk').innerHTML = side('desk', 'Claude Desktop', 'Its own sign-in', dcur && (dcur.email || dcur.label), sideBody(da, dcur && dcur.usage, now), ...desk);
}

// The panels' charts are drawn at their width: draw them again at the new one.
addEventListener('resize', () => S && renderSides());
