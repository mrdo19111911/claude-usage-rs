// The "Settings" card: inputs, recent events, diagnostics.
// Parts: renderSettings.

function renderSettings() {
  const s = S.settings;
  if (document.activeElement?.tagName !== 'INPUT') {
    $('#autoSwitch').checked = s.autoSwitch;
    $('#threshold').value = s.threshold;
    $('#weekThreshold').value = s.weekThreshold;
    $('#pollSeconds').value = s.pollSeconds;
  }
  $('#events').innerHTML = S.events.map(e => `<li>${hhmm(e.t)} — ${esc(e.msg)}</li>`).join('');
  $('#import').style.display = S.demo ? 'none' : '';
  $('#diag').textContent =
    `Claude Code login read from: ${S.loginFile} (${S.loginFound ? 'found' : 'NOT FOUND'})\n` +
    `Claude Code is signed in as: ${S.loginEmail || '?'}  (from ${S.claudeJson})\n` +
    `Last poll: ${S.lastPollAt ? new Date(S.lastPollAt).toLocaleTimeString() : 'never'}\n\n` +
    S.accounts.map(a => `${a.email}${a.id === S.activeId ? '  [ACTIVE]' : ''}\n  error: ${a.error || '-'}\n  Anthropic said: ${JSON.stringify(a.raw)}`).join('\n\n');
}
