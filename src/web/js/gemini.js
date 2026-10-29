// The "Claude Code on Gemini" card: once a key is saved, Use this (a window running
// cgemini), and folded away the model, the `cgemini` command to copy; Remove. Adding
// the key (the dialog) is in gemini_dialog.js.
// Parts: KEY_URL, gemSaved, gemLast, gem, gemFree, renderGemini, gemReady, gemModels;
// #gemBody click/change.

const KEY_URL = 'https://aistudio.google.com/app/apikey';
let gemSaved = false;   // the key was saved from this page: say what to do next
let gemLast = '';       // the card's last HTML (redrawn only when it changes)
const gem = () => S && S.gemini || null;   // an older backend sends no `gemini`
// Only Flash and Flash Lite models are known to have a free tier (docs/specs/gemini.md):
// any other may cost money, or fail at once on a free key.
const gemFree = m => /flash/.test(m || '');

function renderGemini() {
  const g = gem();
  gemDlgSync(g);   // the dialog's status line; a key just saved closes it (sets gemSaved)
  $('#gemAdd').hidden = !!g;
  if (!g) gemSaved = false;
  const html = g ? gemReady(g) : '';
  if (html === gemLast) return;   // keep an open model menu and focus as they are
  $('#gemBody').innerHTML = gemLast = html;
}

// One button for someone who has never opened a terminal; the rest folded away
// (docs/specs/gemini.md, "Onboarding").
function gemReady(g) {
  const cmd = esc(g.command || 'cgemini');
  const done = gemSaved ? '<p class="gem-ok" role="status">All set! Press <b>Use this</b> to start.</p>' : '';
  const fast = g.fastModel ? `<span class="note">Small side jobs use: ${esc(g.fastModel)}</span>` : '';
  const free = gemFree(g.model) && (!g.fastModel || gemFree(g.fastModel));
  return `${done}<div class="gem-ready">
    <p><span class="dot"></span>${free ? 'Gemini is ready (free).' : 'Gemini is ready.'}</p>
    <p><button class="btn primary" data-gopen="1">Use this</button></p>
    <p class="note${free ? '' : ' gem-warn'}">${free ? 'Free has a daily limit and can be slower than Claude.'
      : 'This model may not be free. Pick a Flash one under Advanced to stay free.'}</p>
    <details class="gem-adv"><summary>Advanced</summary>
      <label class="gem-model">Model <select id="gemModel">${gemModels(g)}</select></label>${fast}
      <p>Or type <code>${cmd}</code> in a new terminal <button class="btn" data-gcopy="${cmd}">Copy</button></p>
    </details>
    <div><button class="btn" data-gremove="1">Remove</button></div>
  </div>`;
}

// The key's Gemini models; the one in use is always there. Those not known to be free say so.
function gemModels(g) {
  const list = [...(g.models || [])];
  if (g.model && !list.includes(g.model)) list.unshift(g.model);
  return list.map(m => `<option value="${esc(m)}"${m === g.model ? ' selected' : ''}>${esc(m)}${gemFree(m) ? '' : ' (may cost money)'}</option>`)
    .join('');
}

$('#gemBody').addEventListener('change', e => {
  if (e.target.id === 'gemModel')
    act(api('/api/gemini/model', { method: 'POST', body: JSON.stringify({ model: e.target.value }) }));
});
$('#gemBody').addEventListener('click', e => {
  const copy = e.target.closest('[data-gcopy]');
  if (e.target.closest('[data-gopen]')) {
    act(api('/api/gemini/open', { method: 'POST' }));
  } else if (copy) {
    navigator.clipboard.writeText(copy.dataset.gcopy).then(() => {
      copy.textContent = 'Copied';
      setTimeout(() => (copy.textContent = 'Copy'), 1500);
    }).catch(() => alert('Copy did not work: type ' + copy.dataset.gcopy));
  } else if (e.target.closest('[data-gremove]')
    && confirm('Remove the Gemini key from this computer? cgemini stops working until you add a key again.')) {
    act(api('/api/gemini', { method: 'DELETE' }));
  }
});
