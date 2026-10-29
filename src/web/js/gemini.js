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

// Said for someone who has never opened a terminal (docs/specs/gemini.md, "Onboarding").
function gemReady(g) {
  const cmd = esc(g.command || 'cgemini');
  const done = gemSaved ? '<p class="gem-ok" role="status">All set! Follow the steps below.</p>' : '';
  const fast = g.fastModel ? `<span class="note">Small side jobs use: ${esc(g.fastModel)}</span>` : '';
  return `${done}<div class="gem-ready">
    <p><span class="dot"></span>Done: your free Gemini code is saved <span class="note">(it ends in …${esc(g.keyEnd || '')})</span>.</p>
    <p><b>How to use it:</b></p>
    <ol class="gem-use">
      <li>Open <b>PowerShell</b>: press the Windows key, type <i>PowerShell</i>, press <b>Enter</b>.</li>
      <li>Type <code>${cmd}</code> and press <b>Enter</b>. <button class="btn" data-gcopy="${cmd}">Copy</button></li>
      <li>Claude Code starts as usual, but Gemini answers instead of Claude. Type what you want done.</li>
    </ol>
    <p class="note">Free has a daily limit: when it runs out, try again tomorrow. It can be slower than Claude.</p>
    <details class="gem-adv"><summary>Advanced: which Gemini model</summary>
      <label class="gem-model">Model <select id="gemModel">${gemModels(g)}</select></label>${fast}
    </details>
    <div><button class="btn" data-gremove="1">Remove the code</button></div>
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
