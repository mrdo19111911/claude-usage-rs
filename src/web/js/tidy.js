// A tidy page (docs/specs/clean-page.md): an account's problem as a short label in plain
// words, its one sentence on a click (the raw text stays in Diagnostics); the Settings card
// behind the top bar's ⚙ button.
// Parts: errorLabel, errBadge, errOpen, the click handling.

// Accounts whose sentence is open: kept across the page's redraws, forgotten on reload.
const errOpen = new Set();

// The raw error, said in plain words: { label, say, signIn }.
function errorLabel(raw) {
  const t = String(raw || '').toLowerCase();
  if (t.includes('429') || t.includes('slow down')) {
    const m = t.match(/(\d+)\s*min/);
    const when = m ? `in ${m[1]} min` : 'in a few minutes';
    return { label: '⏳ Paused for a while', say: `Anthropic asked us to check less often. The figures shown stay as they were; the next check is ${when}.` };
  }
  if (t.includes('/login') || t.includes('refresh token')) {
    return { label: '⚠ Sign in again', signIn: true, say: "This account's sign-in has ended. Sign in once more to keep seeing its usage and switching to it." };
  }
  if (t.includes('renews') || t.includes('token expired')) {
    return { label: '⏳ Waiting for Claude Code', say: 'The sign-in needs renewing; Claude Code does it the next time you use it on this account.' };
  }
  if (/http 5\d\d|\(5\d\d\)/.test(t)) {
    return { label: '⚠ Anthropic is busy', say: "Anthropic's servers had a problem. It tries again by itself." };
  }
  if (/dns|connect|timed out|network|resolve/.test(t)) {
    return { label: '⚠ No connection', say: "Couldn't reach Anthropic. Check the internet connection; it tries again by itself." };
  }
  return { label: "⚠ Can't check now", say: "Couldn't check this account just now. It tries again by itself; details under Settings → Advanced → Diagnostics." };
}

// The label button, and its sentence under it (hidden until clicked).
function errBadge(a) {
  if (!a || !a.error) return '';
  const e = errorLabel(a.error), open = errOpen.has(a.id);
  const wait = e.label.startsWith('⏳') ? ' calm' : '';
  const signIn = e.signIn ? ` <button class="btn primary" data-signin="${esc(a.email || '')}">Sign in</button>` : '';
  return `<div class="errbox"><button class="err-pill${wait}" data-errmore="${esc(a.id)}" aria-expanded="${open}">${e.label}</button>
    <div class="err-say note"${open ? '' : ' hidden'}>${esc(e.say)}${signIn}</div></div>`;
}

document.addEventListener('click', ev => {
  const pill = ev.target.closest('[data-errmore]');
  if (pill) {
    const id = pill.dataset.errmore, open = !errOpen.has(id);
    open ? errOpen.add(id) : errOpen.delete(id);
    pill.setAttribute('aria-expanded', open);
    pill.nextElementSibling.hidden = !open;
    return;
  }
  const btn = ev.target.closest('#settingsBtn');
  if (!btn) return;
  const card = document.querySelector('#settingsCard'), open = card.hidden;
  card.hidden = !open;
  btn.setAttribute('aria-expanded', open);
  if (open) card.scrollIntoView({ behavior: 'smooth', block: 'start' });
});
