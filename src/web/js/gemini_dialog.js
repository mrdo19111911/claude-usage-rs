// The dialog that adds a free Gemini key (docs/specs/gemini.md, "Copy to add"). While it
// is open the app watches the clipboard: the user signs in on Google's page and presses
// Copy next to their access code; the app picks it up, checks it with Google and saves it.
// Meanwhile the page asks for the state every second. Pasting by hand stays, under "Or
// paste it here". The key is only ever in the field: never in a URL, storage or the log.
// Parts: gemPoll, gemIsOpen, gemRenew, gemNoWatch, gemLine, gemWatch, gemTick, gemWatched,
// openGemDlg, gemCannot, gemDlgSync, gemLineFor, gemStop, gemErr, gemBusy, gemDone,
// gemClear; #gemAdd #gemShow #gemCancel #gemDone onclick, #gemKey keydown,
// #gemPaste toggle, #gemDlg close/cancel.

let gemPoll = 0;          // the 1 s timer while the dialog is open
let gemIsOpen = false;
let gemRenew = 0;         // when `on: true` was last sent (Date.now())
let gemNoWatch = false;   // this app cannot watch the clipboard (older, or --demo): paste only
let gemLine = '';         // the status line's last HTML (rewritten only when it changes)

const gemWatch = on => api('/api/gemini/watch', { method: 'POST', body: JSON.stringify({ on }) });
const gemTick = () => api('/api/state').then(setState).catch(() => {});
const gemWatched = j => { if (gemIsOpen) setState(j); };

function openGemDlg() {
  gemErr('');
  gemBusy(false);
  gemNoWatch = false;
  gemIsOpen = true;
  if (!$('#gemDlg').open) $('#gemDlg').showModal();
  $('#gemTitle').focus();
  gemRenew = Date.now();
  gemWatch(true).then(gemWatched, gemCannot);
  clearInterval(gemPoll);
  gemPoll = setInterval(gemTick, 1e3);
  gemDlgSync(gem());
}

// No clipboard watch here: say so and open the paste part.
function gemCannot() {
  if (!gemIsOpen) return;
  gemNoWatch = true;
  clearInterval(gemPoll);
  $('#gemPaste').open = true;
  gemDlgSync(gem());
}

// Called by renderGemini on every new state while the dialog is open.
function gemDlgSync(g) {
  if (!gemIsOpen) return;
  if (g) { gemSaved = true; $('#gemDlg').close(); return; }   // saved: the card says what next
  const w = S && S.geminiWatch || null;
  // The app stops watching after 10 minutes: ask again while the dialog is still open.
  if (!gemNoWatch && (!w || w.until - (Date.now() - clockSkew) < 60e3) && Date.now() - gemRenew > 10e3) {
    gemRenew = Date.now();
    gemWatch(true).then(gemWatched, () => {});
  }
  const html = gemLineFor(w);
  if (html !== gemLine) $('#gemWatch').innerHTML = gemLine = html;
}

function gemLineFor(w) {
  if (gemNoWatch) return 'This app cannot pick it up by itself here: paste it below.';
  const st = w ? w.state : 'waiting';
  if (st === 'checking') return '<span class="gem-spin" aria-hidden="true"></span>Checking it with Google…';
  if (st === 'error')
    return `<span class="err">${esc(w.error || 'Google did not accept it.')} Copy it again or try another.</span>`;
  return '<span class="gem-pulse" aria-hidden="true"></span>Waiting for you to press <b>Copy</b>…';
}

function gemErr(msg) { $('#gemErr').textContent = msg; }

function gemBusy(on) {
  $('#gemDone').disabled = on;
  $('#gemKey').readOnly = on;
  $('#gemDone').textContent = on ? 'Checking…' : 'Done';
  $('#gemBusy').textContent = on ? 'Checking the key with Google (up to 10 s)…' : '';
}

// Pasted by hand: Google checks it (the app asks it for the key's models); a bad one comes
// back with Google's own message.
function gemDone() {
  const key = $('#gemKey').value.trim();
  if (!key) { gemErr("Paste the access code from Google's page first."); $('#gemKey').focus(); return; }
  gemErr('');
  gemBusy(true);
  api('/api/gemini/key', { method: 'POST', body: JSON.stringify({ key }) })
    .then(j => { gemSaved = true; $('#gemDlg').close(); setState(j); })
    .catch(e => {
      gemErr(e instanceof SyntaxError ? 'The app gave no answer it understands: is it up to date?'
        : e instanceof TypeError ? 'The dashboard is not answering: is the app running?'
        : e.message || 'Google did not accept this access code.');
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
$('#gemPaste').addEventListener('toggle', () => { if ($('#gemPaste').open) $('#gemKey').focus(); });

// Nothing of the key is left in the page.
function gemClear() {
  $('#gemKey').value = '';
  $('#gemKey').type = 'password';
  $('#gemShow').textContent = 'Show';
  $('#gemShow').setAttribute('aria-pressed', 'false');
  $('#gemPaste').open = false;
  $('#gemWatch').innerHTML = gemLine = '';
  gemErr('');
}

// Closed any way (saved, Done, Cancel, Esc): the watch and the 1 s polling stop. Esc sends
// `cancel`, and some browsers then no `close`, so both.
function gemStop() {
  gemClear();
  if (!gemIsOpen) return;
  gemIsOpen = false;
  clearInterval(gemPoll);
  gemWatch(false).catch(() => {});
}
$('#gemDlg').addEventListener('close', gemStop);
$('#gemDlg').addEventListener('cancel', gemStop);
