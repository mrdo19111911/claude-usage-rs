# The dashboard in its own window (WebView2), not a browser tab

Why: the user asked for the dashboard to look like a large gadget, not a page in the
browser: no tab, no address bar, no visible server. The page itself (src/web) does not
change; it is shown in a window of SCAS with an embedded WebView2 (Edge's engine, part of
Windows 10/11). Prior art and choice: `docs/specs/dashboard-window-prior-art.md`.

## What the user sees (Windows)

- "Open dashboard" (tray click, tray menu, gadget footer, a second launch, the start of
  the app) opens one window titled "Smartlog Claude Auto Switch", with the app's icon and a
  dark title bar; the page fills it. Opened again, the same window comes to the front
  (restored if minimised).
- First size: 1180 x 820 at 100 % (scaled with the screen's DPI), centred on the screen's
  work area, never bigger than it. Moved or resized, the window opens there next time
  (`dashboard_window.json` in the data folder), unless that place is now off every screen.
- Closing the window frees the WebView (its memory); the app goes on in the tray.
- Links that leave the dashboard (new windows: Help, "What's new", Google's key page;
  any page not on `http://127.0.0.1:<port>` or `http://localhost:<port>`) open in the
  default browser. The sign-in pages for adding an account already open there.
- No WebView2 runtime (or it fails to start): the dashboard opens in the browser as before.
- The WebView's data (cache, local storage) is kept in `<data folder>\webview`, not next
  to the exe in Program Files.
- macOS and `--no-open`: unchanged.

## Modules

- `src/tray/dash_rect.rs` (pure): `first_rect(saved, work, scale) -> Rect` (the size
  rule above) and `rect_json` / `rect_from_json` for `dashboard_window.json`.
  Tests: `dash_rect_tests.rs`.
- `src/tray/dash_nav.rs` (pure): `stays_inside(url, port) -> bool`, whether a navigation
  stays in the window. Tests: `dash_nav_tests.rs`.
- `src/tray/dash_window.rs`: the Win32 window and the wry WebView in it; `open_dashboard`
  (events.rs) calls `dash_window::open`, falling back to `crate::open_browser`.

## Done when

- Tests: `first_rect` (no file: default centred; scaled at 150 %; clipped to a small work
  area; saved and on screen: kept; saved but off screen: default); the JSON round trip;
  `stays_inside` (the dashboard's own pages on 127.0.0.1 and localhost at the port: yes;
  another port, https sites, `about:blank`, a look-alike host: no).
- The window opens from the tray and the gadget, shows the live dashboard, resizes, comes
  back to the front when opened again, and remembers its place; all tests green.
