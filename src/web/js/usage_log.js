// The Settings card's "Usage log" row: how many lines the log holds, since when, and an
// Export link (a .jsonl download, accounts named acc1, acc2…, no email).
// Parts: logText, renderUsageLog.

// "1,234 lines since 28 Oct", or "nothing yet" with no line.
function logText(log) {
  const n = log.lines || 0;
  if (!n) return 'nothing yet';
  const since = log.since ? ` since ${new Date(log.since).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })}` : '';
  return `${n.toLocaleString('en-US')} ${n === 1 ? 'line' : 'lines'}${since}`;
}

// An older backend sends no `usageLog`: no row then. Never in the demo (no log there).
function renderUsageLog() {
  const log = S.usageLog;
  $('#logRow').hidden = !log || !!S.demo;
  if (!log) return;
  const n = log.lines || 0;
  $('#logStatus').textContent = '· ' + logText(log);
  $('#logExport').hidden = !n;
  $('#logExport').href = '/api/usage-log/export';
}
