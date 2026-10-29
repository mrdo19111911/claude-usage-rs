// The "Claude accounts" section's right list: Claude Desktop.
// Parts: renderDesk.

// The list's HTML (docs/specs/modules.md, "The Claude accounts section"): one listRow
// per saved Desktop account, then CLI accounts with no Desktop sign-in yet, muted, with
// "Use here (first time)". Desktop not found, or the demo: a note instead.
function renderDesk() {
  const D = S.desktop, busy = D.switching ? 'disabled' : '';
  const note = t => `<div class="note" style="margin-top:10px">${t}</div>`;
  if (S.demo) return note('Not available in the demo.');
  if (!D.found) return note('Claude Desktop was not found on this computer.');
  const rows = accountRows();
  const saved = rows.filter(r => r.d).map(({ a, d }) => {
    const email = (a && a.email) || d.email || d.label || '';
    const forget = d.signedIn ? ''
      : `<div class="row-actions"><button class="btn" data-dremove="${d.uuid}" title="Remove">✕</button></div>`;
    const left = `<div><div class="email">${d.signedIn ? '<span class="dot"></span>' : ''}${esc(email)}</div>`
      + `<div class="meta">${a ? esc(a.plan) : 'no Claude CLI login'}</div>${forget}</div>`;
    const right = d.signedIn ? '<div class="on">✓ In use</div>'
      : d.hasSignIn ? `<button class="btn" data-dswitch="${d.uuid}" ${busy}>Use here</button>`
      : `<button class="btn" data-dfirst="${esc(email)}" ${busy}>Use here (first time)</button>`;
    return listRow(d.signedIn ? 'inuse' : '', left, right, a ? a.usage : d.usage);
  });
  const fresh = rows.filter(r => !r.d).map(({ a }) => listRow('muted',
    `<div><div class="email">${esc(a.email)}</div><div class="meta">not signed in to Claude Desktop yet</div></div>`,
    `<button class="btn" data-dfirst="${esc(a.email)}" ${busy}>Use here (first time)</button>`, a.usage));
  return saved.concat(fresh).join('') || note('No accounts yet.');
}
