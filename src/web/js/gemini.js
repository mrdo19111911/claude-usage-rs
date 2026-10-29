// The "Claude Code on Gemini" card: add a free Google AI Studio key (a dialog with three
// steps), then the model in use, the `cgemini` command to copy, and Remove.
// The key is only ever in the dialog's field: never in a URL, storage or the log.
// Parts: KEY_URL, gemSaved, gemLast, gem, renderGemini, gemReady, gemModels,
// openGemDlg, gemErr, gemBusy, gemDone, gemClear; #gemAdd #gemShow #gemCancel #gemDone onclick,
// #gemKey keydown, #gemDlg close/cancel, #gemBody click/change.

const KEY_URL = 'https://aistudio.google.com/app/apikey';
let gemSaved = false;   // the key was saved from this page: say what to do next
let gemLast = '';       // the card's last HTML (redrawn only when it changes)
const gem = () => S && S.gemini || null;   // an older backend sends no `gemini`

function renderGemini() {
  const g = gem();
  $('#gemAdd').hidden = !!g;
  if (!g) gemSaved = false;
  const html = g ? gemReady(g) : '';
  if (html === gemLast) return;   // keep an open model menu and focus as they are
  $('#gemBody').innerHTML = gemLast = html;
}

function gemReady(g) {
  const cmd = esc(g.command || 'cgemini');
  const done = gemSaved
    ? `<p class="gem-ok" role="status">Your Gemini account is ready. Open a new terminal and type <code>${cmd}</code> to start coding.</p>` : '';
  const fast = g.fastModel ? ` <span class="note">(quick tasks: ${esc(g.fastModel)})</span>` : '';
  return `${done}<div class="gem-ready">
    <div><span class="dot"></span><b>Ready</b> <span class="note">· key …${esc(g.keyEnd || '')}</span></div>
    <label class="gem-model">Model <select id="gemModel">${gemModels(g)}</select>${fast}</label>
    <div class="gem-cmd"><span>Open a new terminal and type <code>${cmd}</code></span>
      <button class="btn" data-gcopy="${cmd}">Copy</button></div>
    <div><button class="btn" data-gremove="1">Remove</button></div>
  </div>`;
}

// The key's Gemini models; the one in use is always there.
function gemModels(g) {
  const list = [...(g.models || [])];
  if (g.model && !list.includes(g.model)) list.unshift(g.model);
  return list.map(m => `<option value="${esc(m)}"${m === g.model ? ' selected' : ''}>${esc(m)}</option>`).join('');
}

function openGemDlg() {
  gemErr('');
  gemBusy(false);
  if (!$('#gemDlg').open) $('#gemDlg').showModal();
  $('#gemKey').focus();
}

function gemErr(msg) { $('#gemErr').textContent = msg; }

function gemBusy(on) {
  $('#gemDone').disabled = on;
  $('#gemKey').readOnly = on;
  $('#gemDone').textContent = on ? 'Checking…' : 'Done';
  $('#gemBusy').textContent = on ? 'Checking the key with Google (up to 10 s)…' : '';
}

// Google checks the key (the app asks it for the key's models); a bad one comes back
// with Google's own message.
function gemDone() {
  const key = $('#gemKey').value.trim();
  if (!key) { gemErr('Paste the key from Google AI Studio first.'); $('#gemKey').focus(); return; }
  gemErr('');
  gemBusy(true);
  api('/api/gemini/key', { method: 'POST', body: JSON.stringify({ key }) })
    .then(j => { gemSaved = true; $('#gemDlg').close(); setState(j); })
    .catch(e => {
      gemErr(e instanceof SyntaxError ? 'The app gave no answer it understands: is it up to date?'
        : e instanceof TypeError ? 'The dashboard is not answering: is the app running?'
        : e.message || 'Google did not accept this key.');
      $('#gemKey').focus();
    })
    .finally(() => gemBusy(false));
}

$('#gemAdd').onclick = () => {
  window.open(KEY_URL, '_blank', 'noopener');
  openGemDlg();
};
$('#gemShow').onclick = () => {
  const show = $('#gemKey').type === 'password';
  $('#gemKey').type = show ? 'text' : 'password';
  $('#gemShow').textContent = show ? 'Hide' : 'Show';
  $('#gemShow').setAttribute('aria-pressed', show);
};
$('#gemCancel').onclick = () => $('#gemDlg').close();
$('#gemDone').onclick = gemDone;
$('#gemKey').addEventListener('keydown', e => { if (e.key === 'Enter' && !$('#gemDone').disabled) gemDone(); });
// Closed any way (Done, Cancel, Esc): nothing of the key is left in the page. Esc sends
// `cancel`, and some browsers then no `close`, so both.
function gemClear() {
  $('#gemKey').value = '';
  $('#gemKey').type = 'password';
  $('#gemShow').textContent = 'Show';
  $('#gemShow').setAttribute('aria-pressed', 'false');
  gemErr('');
}
$('#gemDlg').addEventListener('close', gemClear);
$('#gemDlg').addEventListener('cancel', gemClear);

$('#gemBody').addEventListener('change', e => {
  if (e.target.id === 'gemModel')
    act(api('/api/gemini/model', { method: 'POST', body: JSON.stringify({ model: e.target.value }) }));
});
$('#gemBody').addEventListener('click', e => {
  const copy = e.target.closest('[data-gcopy]');
  if (copy) {
    navigator.clipboard.writeText(copy.dataset.gcopy).then(() => {
      copy.textContent = 'Copied';
      setTimeout(() => (copy.textContent = 'Copy'), 1500);
    }).catch(() => alert('Copy did not work: type ' + copy.dataset.gcopy));
  } else if (e.target.closest('[data-gremove]')
    && confirm('Remove the Gemini key from this computer? cgemini stops working until you add a key again.')) {
    act(api('/api/gemini', { method: 'DELETE' }));
  }
});
