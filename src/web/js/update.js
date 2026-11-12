// The version and updates: the top bar (version, "Update to X" pill) and the Settings
// card's "Updates" row (status, Check / Install buttons, "Install updates automatically").
// Parts: BUSY, upd, installs, updText, renderUpdate, fastPoll, installUpdate; #updPill, #updInstall,
// #updCheck onclick; #autoUpdate onchange.

const BUSY = ['checking', 'downloading', 'installing'];
// An older backend sends no `update`: treat it as never checked.
const upd = () => (S && S.update) || { status: 'idle' };
// macOS builds do not install (the setup is a Windows .exe): the release page instead.
const installs = () => S.canInstallUpdates !== false;

function updText(u) {
  const when = u.checkedAt ? ` · checked <span data-ago="${u.checkedAt}">${ago(u.checkedAt)}</span>` : '';
  const news = u.url && /^https:\/\//.test(u.url) ? ` · <a href="${esc(u.url)}" target="_blank" rel="noopener">What's new</a>` : '';
  switch (u.status) {
    case 'checking': return 'Checking…';
    case 'downloading': return `Downloading ${esc(u.latest || 'the update')}…`;
    case 'installing': return 'Installing… the app will restart';
    case 'available': return `Version ${esc(u.latest)} available${news}`;
    case 'upToDate': return 'Up to date' + when;
    case 'error': return `<span class="err">${esc(u.error || 'The update check failed')}</span>${when}`;
    // Idle after a restart: the last check is remembered, its answer is not.
    default: return u.checkedAt ? `Last checked <span data-ago="${u.checkedAt}">${ago(u.checkedAt)}</span>` : 'Never checked';
  }
}

function renderUpdate() {
  const u = upd(), v = S.version || '';
  const can = u.status === 'available' && !!u.latest && !S.demo;
  $('#version').textContent = v;
  $('#updVer').textContent = v || '?';
  $('#updPill').hidden = !can;
  $('#updInstall').hidden = !can || !installs();
  $('#autoUpdate').closest('label').hidden = !installs();
  if (can) {
    $('#updPill').textContent = `Update to ${u.latest}`;
    $('#updPill').title = installs() ? '' : 'Opens the release page to download it';
    $('#updInstall').textContent = `Install ${u.latest}`;
  }
  $('#updRow').hidden = !!S.demo;
  $('#updStatus').innerHTML = updText(u);
  $('#updCheck').disabled = BUSY.includes(u.status);
  if (document.activeElement !== $('#autoUpdate')) $('#autoUpdate').checked = S.settings.autoUpdate !== false;
  fastPoll(BUSY.includes(u.status));
}

// Every 2 s while a check, download or install runs (the 10 s load in main.js goes on).
// While installing, the app closes: say so when it stops answering.
let updTimer = null;
function fastPoll(on) {
  if (on && !updTimer) {
    updTimer = setInterval(() => api('/api/state').then(setState).catch(() => {
      if (upd().status === 'installing')
        $('#updStatus').textContent = 'Restarting… if this page does not come back, open the dashboard from the tray icon.';
    }), 2e3);
  } else if (!on && updTimer) {
    clearInterval(updTimer);
    updTimer = null;
  }
}

function installUpdate() {
  const u = upd(), v = u.latest;
  if (!installs()) {
    if (u.url && /^https:\/\//.test(u.url)) window.open(u.url, '_blank', 'noopener');
    return;
  }
  if (!confirm(`Install Claude Usage ${v} now? The dashboard closes and opens again in a moment.`)) return;
  api('/api/update/install', { method: 'POST' }).then(load).catch(e => alert(e.message));
}

$('#updPill').onclick = installUpdate;
$('#updInstall').onclick = installUpdate;
$('#updCheck').onclick = () => {
  $('#updCheck').disabled = true;
  api('/api/update/check', { method: 'POST' }).then(load)
    .catch(e => { $('#updCheck').disabled = false; alert(e.message); });
};
$('#autoUpdate').onchange = () =>
  act(api('/api/settings', { method: 'POST', body: JSON.stringify({ autoUpdate: $('#autoUpdate').checked }) }));
