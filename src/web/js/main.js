// Loaded last: drawing, buttons and the load loop.
// Parts: render, the width watcher (renderCards is in card.js, renderUpdate is in update.js, renderUsageLog in usage_log.js, renderGemini in gemini.js), setState, load, act; #viewToggle, #refresh, #import, #exit onclick;
// autoSwitch/threshold/weekThreshold/shareDesktopSessions/useSoonestReset/renewDesktop onchange; loginUrl, signIn, #cliAdd onclick;
// #accts click, #sides click, run (its ✕ removes both logins), #accts change (both switches); load() and the two timers.

function render() { if (!S) return; renderSides(); renderRows(); renderSettings(); renderUpdate(); renderUsageLog(); renderGemini(); }
function setState(j) { S = j; clockSkew = Date.now() - j.now; render(); }
const load = () => api('/api/state').then(setState).catch(() => {});
const act = p => p.then(setState).catch(e => alert(e.message));
// The charts are drawn at their width (docs/specs/account-cards.md, "Less space" 4, 6):
// draw them again when the page's width changes (not always sent as a resize event).
let drawnWidth = 0;
new ResizeObserver(([e]) => {
  const w = Math.round(e.contentRect.width);
  if (w === drawnWidth) return;
  drawnWidth = w;
  if (S) { renderSides(); renderCards(); }
}).observe($('main'));

$('#viewToggle').onclick = () => { tableView = !tableView; $('#viewToggle').textContent = tableView ? 'Chart view' : 'Table view'; render(); };
$('#refresh').onclick = () => act(api('/api/refresh', { method: 'POST' }));
$('#import').onclick = () => act(api('/api/import', { method: 'POST' }));
$('#exit').onclick = () => {
  if (!confirm('Close Smartlog Claude Auto Switch? Usage checks and auto-switch stop until you open it again.')) return;
  api('/api/exit', { method: 'POST' }).then(() => {
    document.querySelector('main').innerHTML = '<section class="card"><h2>App stopped</h2><p class="note">Open Smartlog Claude Auto Switch from the Start menu to start it again.</p></section>';
  }).catch(e => alert(e.message));
};
for (const id of ['autoSwitch', 'threshold', 'weekThreshold', 'shareDesktopSessions', 'useSoonestReset', 'renewDesktop']) {
  $('#' + id).onchange = () => act(api('/api/settings', { method: 'POST', body: JSON.stringify({
    autoSwitch: $('#autoSwitch').checked, threshold: +$('#threshold').value,
    weekThreshold: +$('#weekThreshold').value,
    shareDesktopSessions: $('#shareDesktopSessions').checked, useSoonestReset: $('#useSoonestReset').checked,
    renewDesktop: $('#renewDesktop').checked }) }));
}
// The sign-in page opens in the default browser; the account is added when it comes back.
// Claude Code keeps the login it uses now.
let loginUrl = '';
const signIn = email => api('/api/login', { method: 'POST', body: JSON.stringify({ email }) })
  .then(r => { loginUrl = r.url; return api('/api/state'); }).then(setState).catch(e => alert(e.message));
$('#cliAdd').onclick = () => signIn('');
// Any account: Claude Desktop opens signed out and saves whichever one signs in there.
$('#deskAdd').onclick = () => run({ dfirst: '' });
$('#accts').addEventListener('click', e => {
  // "Check now" on a card: read that one account's usage right away. The state
  // comes back and redraws the card, so the "Checking…" label is replaced then.
  const c = e.target.closest('[data-check]');
  if (c) {
    c.disabled = true; c.textContent = 'Checking…';
    act(api(`/api/accounts/${c.dataset.check}/check`, { method: 'POST' }));
    return;
  }
  const t = e.target.closest('[data-activate],[data-remove],[data-signin],[data-dswitch],[data-dfirst],[data-dremove]');
  if (t) run(t.dataset);
});
// The panels' Switch buttons: the menu holds "action:value", the same actions as the table's.
$('#sides').addEventListener('click', e => {
  const b = e.target.closest('[data-side]');
  if (!b) return;
  const [k, v] = b.parentNode.querySelector('select').value.split(/:(.*)/);
  run({ [k]: v });
  b.blur();
});
function run(ds) {
  if (ds.activate) act(api(`/api/accounts/${ds.activate}/activate`, { method: 'POST' }));
  else if (ds.signin !== undefined) signIn(ds.signin);
  else if (ds.dswitch) {
    const d = S.desktop.accounts.find(x => x.uuid === ds.dswitch);
    if (confirm(`Claude Desktop will restart signed in as ${d.email || d.label}. Any reply in progress is cut off. Continue?`))
      act(api(`/api/desktop/${d.uuid}/switch`, { method: 'POST' }));
  } else if (ds.dfirst !== undefined) {
    if (confirm(`Claude Desktop will restart signed out. Any reply in progress is cut off. Then sign in there with ${ds.dfirst || 'the account to add'}. Continue?`))
      act(api('/api/desktop/add', { method: 'POST', body: JSON.stringify({ email: ds.dfirst }) }));
  } else if (ds.remove || ds.dremove) {
    // A card's ✕: its Claude Code sign-in and its saved Desktop sign-in, each where not in use.
    if (!confirm('Remove this account from the app? (Your Claude account is not deleted.)')) return;
    let p = Promise.resolve();
    if (ds.remove) p = p.then(() => api(`/api/accounts/${ds.remove}`, { method: 'DELETE' }));
    if (ds.dremove) p = p.then(() => api(`/api/desktop/${ds.dremove}`, { method: 'DELETE' }));
    act(p);
  }
}
// The cards' switches: may auto-switch move Claude Code / Claude Desktop to this account.
$('#accts').addEventListener('change', e => {
  const id = e.target.dataset.toggle, uuid = e.target.dataset.dauto, body = JSON.stringify({ enabled: e.target.checked });
  if (id) act(api(`/api/accounts/${id}/toggle`, { method: 'POST', body }));
  else if (uuid) act(api(`/api/desktop/${uuid}/auto`, { method: 'POST', body }));
});

load();
setInterval(load, 10e3);
setInterval(() => document.querySelectorAll('[data-ago]').forEach(el => (el.textContent = ago(+el.dataset.ago))), 1e3);
