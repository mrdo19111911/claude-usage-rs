# Showing the dashboard in a WebView2 window: prior art (Nov 2026)

The tray runs its own Win32 loop through windows-sys (no tao/winit). Options found:

| Project | How | Fit here | Cost |
|---|---|---|---|
| **wry 0.57.2** (tauri-apps/wry) | `WebViewBuilder::build(&parent)` on any `HasWindowHandle`; wraps webview2-com; does COM init (STA), the startup wait (pumps messages), and follows the parent's `WM_SIZE` (subclass) | Best: works on our own HWND and loop | webview2-com, windows 0.62, raw-window-handle; about 1-2 MB in release |
| webview2-com 0.39.1 (wravery/webview2-rs) | Raw COM: environment, controller, `SetBounds`, event tokens | Works, but ~400 lines of our own (its `sample.rs`) | Lighter deps, much more code |
| sopium/webview2 0.1.4, Boscop/web-view 0.7.3 | Old bindings / old engine | No: abandoned since 2021 | - |
| tao + wry, tray-icon examples | tao/winit own the event loop | No: would replace our loop | Large dependency tree |

Choice: **wry on our own top-level window** (`src/tray/dash.rs`). Details from wry's source
(`src/webview2/mod.rs`): it calls `CoInitializeEx(COINIT_APARTMENTTHREADED)` itself;
building blocks in `wait_with_pump`, so window procedures can run again meanwhile (no
`RefCell` borrow held across `build`); a missing runtime is an `Err` from `build` (and from
`wry::webview_version()`), so we fall back to the browser. The data folder is set with
`WebContext::new(Some(dir))`: without it WebView2 writes `scas.exe.WebView2` next to the
exe, which fails in Program Files. External links: `with_new_window_req_handler` →
browser + `Deny`; `with_navigation_handler` → `false` outside the dashboard.
