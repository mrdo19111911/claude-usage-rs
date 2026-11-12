# The gadget: a row opens its chart (spec)

Asked 2026-11-12: "trong cái gadget, nếu ấn vào một acc, có cách nào hiện ra detail hơn, ví
dụ biểu đồ/ đồ thị không?", then picked "Mở rộng ngay trong gadget".

## What the user sees
- A click on an account row ("SWITCH TO" or "USED UP · AVAILABLE AT"), outside its CLI /
  Desktop buttons, opens that row: under it, a chart ~96 px tall (at 1x). A second click on
  the row closes it. One row open at a time: opening another closes the first. Not
  remembered across restarts.
- The chart, the dashboard's two windows side by side in small:
  - left "5-hour", right "Weekly": each from the window's start to its reset (or the last 5 h
    / 7 days to now when the reset is not known), 0–100 %;
  - the usage line from the account's history, in the dashboard's colours (5-hour blue
    `#4c8df6`, weekly orange `#f0653c`), the last figure carried to now, a dot at now;
  - the auto-switch line (the CLI thresholds: `settings.threshold`, `weekThreshold`), dashed;
  - the reset: a green line at the right edge with its time under it ("17:53", "Sat 15:53");
    a reset that has passed and still falls in the span is marked the same way at its time,
    the line dropping to 0 % there (as docs/specs/account-cards.md, "A reset that has passed");
  - "No figures yet" when the account has none.
- Rows show a hand cursor; on the pinned widget a row click is a click, not a drag.
- Left out: an "Open in dashboard" link under the chart (the footer already opens it).
- The hover gadget (shown while the pointer is on the tray icon) opens rows the same way;
  any other click on it still opens the dashboard.

## Parts
- `tray/model.rs`: `Other.history: Vec<(i64, i64, i64)>` (time ms, 5-hour %, weekly %) from
  `/api/state`'s `history`; `Model.settings` thresholds.
- `tray/chart_data.rs` (new, pure, tested): `points(history, k, span, reset, now) -> Vec<(i64,i64)>`
  (window start, passed reset drop, last figure carried to now), `xy(...)` mapping to pixels
  with one min/max pair per pixel column when there are more points than columns.
- `tray/gdip.rs` (new): GDI+ flat API, smooth lines (docs/SO_SANH_GADGET_CHART.md, C).
- `tray/chart_paint.rs` (new): drawing the two panels.
- `tray/rows.rs` (new): the open row, row click targets, resizing both widgets.
- `tray/sections.rs`: rows report their rectangle as a click target; the open row id lives in
  the UI state; layout adds the chart's height under the open row.
- `tray/widget.rs`: a click on a row toggles it and resizes the window.

## Tests (written first)
- `model_tests`: history and thresholds read from a state JSON.
- `chart_tests`: points for a known reset, an unknown one, a passed one; the per-column
  min/max keeps the peaks; x/y stay inside the chart box.
- `buttons_tests`: a click on a row (not on a button) hits that row; on a button, the button.
- The preview test gains a preset with one open row (`target/widget-preview/open_row_*.bmp`).
