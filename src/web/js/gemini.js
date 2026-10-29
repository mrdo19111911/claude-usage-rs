// The "Claude Code on Gemini" card: once a key is saved, the model in use, the `cgemini`
// command to copy, and Remove. Adding the key (the dialog) is in gemini_dialog.js.
// Parts: KEY_URL, gemSaved, gemLast, gem, renderGemini, gemReady, gemModels;
// #gemBody click/change.

const KEY_URL = 'https://aistudio.google.com/app/apikey';
let gemSaved = false;   // the key was saved from this page: say what to do next
let gemLast = '';       // the card's last HTML (redrawn only when it changes)
const gem = () => S && S.gemini || null;   // an older backend sends no `gemini`

function renderGemini() {
  const g = gem();
  gemDlgSync(g);   // the dialog's status line; a key just saved closes it (sets gemSaved)
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
