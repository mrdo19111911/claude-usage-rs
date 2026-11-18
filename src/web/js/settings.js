// The "Settings" card: inputs, recent events, diagnostics.
// Parts: renderSettings.

function renderSettings() {
  const s = S.settings;
  if (document.activeElement?.tagName !== 'INPUT') {
    $('#autoSwitch').checked = s.autoSwitch;
    // Missing on an older backend: on, its default.
    $('#shareDesktopSessions').checked = s.shareDesktopSessions !== false;
    $('#useSoonestReset').checked = s.useSoonestReset !== false;
    $('#renewDesktop').checked = s.renewDesktop !== false;
    $('#threshold').value = s.threshold;
    $('#weekThreshold').value = s.weekThreshold;
    // The polling settings (check-schedule.md); an older backend sends none.
    for (const [id, d] of [['lowMin', 15], ['midMin', 7], ['highMin', 2], ['midPct', 50], ['highPct', 90], ['weekHighPct', 98],['idleMin', 10], ['idleMax', 15]]) $('#' + id).value = s[id] ?? d;
    $('#pollOthers').checked = s.pollOthers === true;
  }
  $('#events').innerHTML = S.events.map(e => `<li>${hhmm(e.t)} — ${esc(e.msg)}</li>`).join('');
  $('#import').style.display = S.demo ? 'none' : '';
  $('#diag').textContent =
    `Claude Code login read from: ${S.loginFile} (${S.loginFound ? 'found' : 'NOT FOUND'})\n` +
    `Claude Code is signed in as: ${S.loginEmail || '?'}  (from ${S.claudeJson})\n` +
    `Last check: ${S.lastPollAt ? new Date(S.lastPollAt).toLocaleTimeString() : 'never'}\n\n` +
    S.accounts.map(a => `${a.email}${a.id === S.activeId ? '  [ACTIVE]' : ''}\n  error: ${a.error || '-'}\n  Anthropic said: ${JSON.stringify(a.raw)}`).join('\n\n');
}
