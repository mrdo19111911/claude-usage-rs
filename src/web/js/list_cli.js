// The "Claude accounts" section's left list: Claude CLI (terminal and VS Code).
// Parts: renderCli.

// The list's HTML (docs/specs/modules.md, "The Claude accounts section"): one listRow
// per CLI login, then the accounts only Claude Desktop knows, muted, with Sign in.
function renderCli() {
  const rows = accountRows().map(({ a, d }) => {
    if (a) {
      const inUse = a.id === S.activeId;
      const [icon, label] = status(a);
      const left = `<label class="switch" title="Auto-switch may use it"><input type="checkbox" data-toggle="${a.id}" ${a.enabled ? 'checked' : ''}><span></span></label>
        <div>
          <div class="email">${inUse ? '<span class="dot"></span>' : ''}${esc(a.email)}</div>
          <div class="meta">${esc(a.plan)}</div>
          <span class="pill">${icon} ${label}</span>${a.error ? `<div class="err">${esc(a.error)}</div>` : ''}${checkLine(a)}
          <div class="row-actions">${inUse ? '' : `<button class="btn" data-remove="${a.id}" title="Remove">✕</button>`}</div>
        </div>`;
      const right = inUse ? '<div class="on">✓ In use</div>' : `<button class="btn" data-activate="${a.id}">Use here</button>`;
      return listRow(inUse ? 'inuse' : '', left, right, a.usage);
    }
    const left = `<div>
        <div class="email">${esc(d.email || d.label)}</div>
        <div class="meta">no Claude CLI login yet</div>
      </div>`;
    const right = `<button class="btn" data-signin="${esc(d.email || '')}">Sign in</button>`;
    return listRow('muted', left, right, d.usage);
  });
  return rows.join('') || '<div class="note" style="margin-top:10px">No accounts yet. Click “+ Add account” and sign in.</div>';
}
