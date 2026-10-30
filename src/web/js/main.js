// Loaded last: drawing, buttons and the load loop.
// Parts: render (renderUpdate is in update.js, renderUsageLog in usage_log.js, renderGemini in gemini.js), setState, load, act; #viewToggle, #refresh, #import, #exit onclick;
// autoSwitch/threshold/weekThreshold/pollSeconds/shareDesktopSessions onchange; loginUrl, signIn, #cliAdd onclick;
// #accts click, #sides click, run, #accts change; load() and the two timers.

function render() { if (!S) return; renderSides(); renderChart(); renderRows(); renderSettings(); renderUpdate(); renderUsageLog(); renderGemini(); }
function setState(j) { S = j; clockSkew = Date.now() - j.now; render(); }
const load = () => api('/api/state').then(setState).catch(() => {});
const act = p => p.then(setState).catch(e => alert(e.message));

$('#viewToggle').onclick = () => { tableView = !tableView; $('#viewToggle').textContent = tableView ? 'Chart view' : 'Table view'; render(); };
$('#refresh').onclick = () => act(api('/api/refresh', { method: 'POST' }));
$('#import').onclick = () => act(api('/api/import', { method: 'POST' }));
$('#exit').onclick = () => {
  if (!confirm('Stop the dashboard? Checks and auto-switch stop until you start it again.')) return;
  api('/api/exit', { method: 'POST' }).then(() => {
    document.querySelector('main').innerHTML = '<section class="card"><h2>Dashboard stopped</h2><p class="note">Start claude-usage.exe again to bring it back.</p></section>';
  }).catch(e => alert(e.message));
};
for (const id of ['autoSwitch', 'threshold', 'weekThreshold', 'pollSeconds', 'shareDesktopSessions']) {
  $('#' + id).onchange = () => act(api('/api/settings', { method: 'POST', body: JSON.stringify({
    autoSwitch: $('#autoSwitch').checked, threshold: +$('#threshold').value,
    weekThreshold: +$('#weekThreshold').value, pollSeconds: +$('#pollSeconds').value,
    shareDesktopSessions: $('#shareDesktopSessions').checked }) }));
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
    if (confirm(`Claude Desktop will close and open again on ${d.email || d.label}. Running tasks stop. Go on?`))
      act(api(`/api/desktop/${d.uuid}/switch`, { method: 'POST' }));
  } else if (ds.dfirst !== undefined) {
    if (confirm(`Claude Desktop will close and open signed out (running tasks stop). Then click “Continue with Google” in it and choose ${ds.dfirst || 'the account'}. Go on?`))
      act(api('/api/desktop/add', { method: 'POST', body: JSON.stringify({ email: ds.dfirst }) }));
  } else if (ds.remove) {
    if (confirm('Remove this account from the dashboard?')) act(api(`/api/accounts/${ds.remove}`, { method: 'DELETE' }));
  } else if (ds.dremove && confirm('Forget this account’s saved Claude Desktop sign-in?')) {
    act(api(`/api/desktop/${ds.dremove}`, { method: 'DELETE' }));
  }
}
$('#accts').addEventListener('change', e => {
  const id = e.target.dataset.toggle;
  if (id) act(api(`/api/accounts/${id}/toggle`, { method: 'POST', body: JSON.stringify({ enabled: e.target.checked }) }));
});

load();
setInterval(load, 10e3);
setInterval(() => document.querySelectorAll('[data-ago]').forEach(el => (el.textContent = ago(+el.dataset.ago))), 1e3);
