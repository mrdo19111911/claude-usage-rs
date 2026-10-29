// The Gemini card and dialog, after review (docs/specs/gemini.md, "Onboarding" and "Copy to
// add"): the clipboard watch ends when the page goes away and after 10 minutes from opening,
// a late answer from an older dialog changes nothing, "free" only for Flash models, and
// `cgemini` in a new terminal.

use super::asset;

fn card_js() -> &'static str {
    asset("/js/gemini.js").expect("gemini.js served").0
}

fn dlg_js() -> &'static str {
    asset("/js/gemini_dialog.js").expect("gemini_dialog.js served").0
}

// The body of `function name` (up to the first line that is only "}").
fn body<'a>(js: &'a str, name: &str) -> &'a str {
    let f = &js[js.find(&format!("function {name}")).unwrap_or_else(|| panic!("function {name}"))..];
    &f[..f.find("\n}").unwrap()]
}

#[test]
fn leaving_the_page_stops_the_watch_with_keepalive() {
    let j = dlg_js();
    // The same api() call, so the same headers (X-CUD) go with it.
    assert!(j.contains("api('/api/gemini/watch', { method: 'POST', keepalive"), "the watch call can be keepalive");
    let i = j.find("addEventListener('pagehide'").expect("a pagehide handler");
    let h = &j[i..i + j[i..].find("});").unwrap()];
    assert!(h.contains("gemIsOpen") && h.contains("gemWatch(false, true)"), "off, keepalive, when open: {h}");
    let i = j.find("addEventListener('pageshow'").expect("a pageshow handler");
    let h = &j[i..i + j[i..].find("});").unwrap()];
    assert!(h.contains("persisted") && h.contains("$('#gemDlg').close()"), "back from the cache: closed: {h}");
}

#[test]
fn a_late_answer_from_an_older_dialog_changes_nothing() {
    let j = dlg_js();
    assert!(j.contains("let gemSess = 0;"), "a dialog session number");
    assert!(body(j, "openGemDlg").contains("gemSess++"), "each opening is a new session");
    assert!(body(j, "gemStop").contains("gemSess++"), "and closing ends it");
    let done = body(j, "gemDone");
    assert!(done.contains("const s = gemSess;"), "the request remembers its session");
    assert!(done.matches("s !== gemSess").count() >= 3, "success, error and busy all check it: {done}");
    assert!(done.contains("setState(j)"), "a key saved is still taken into the state");
    let open = body(j, "openGemDlg");
    assert!(open.contains("s === gemSess && gemCannot()"), "a late watch refusal too: {open}");
}

#[test]
fn the_watch_lasts_ten_minutes_from_opening_at_most() {
    let j = dlg_js();
    assert!(j.contains("const GEM_WATCH_MS = 10 * 60e3;"));
    assert!(body(j, "openGemDlg").contains("gemStart = Date.now()"), "counted from opening");
    let sync = body(j, "gemDlgSync");
    assert!(!sync.contains("< 60e3"), "never renewed to make it longer");
    assert!(sync.contains("Date.now() - gemStart >= GEM_WATCH_MS"), "ends at 10 minutes: {sync}");
    assert!(sync.contains("!w &&"), "asks again only when the app lost the watch: {sync}");
    let up = body(j, "gemTimeUp");
    for s in ["gemWatch(false)", "clearInterval(gemPoll)", "$('#gemPaste').open = true"] {
        assert!(up.contains(s), "time up: {s}");
    }
    assert!(j.contains("Stopped looking after 10 minutes: paste it below."));
    let help = asset("/help.html").unwrap().0;
    assert!(help.contains("(10 minutes at most)"), "help keeps its promise");
}

#[test]
fn free_is_said_only_for_flash_models() {
    let j = card_js();
    assert!(j.contains("const gemFree = m => /flash/.test(m || '');"), "Flash and Flash Lite are the free ones");
    let card = body(j, "gemReady");
    assert!(card.contains("gemFree(g.model) && (!g.fastModel || gemFree(g.fastModel))"), "both models count");
    assert!(card.contains("free ? 'Gemini is ready (free).' : 'Gemini is ready.'"), "{card}");
    assert!(card.contains("This model may not be free. Pick a Flash one under Advanced to stay free."));
    assert!(body(j, "gemModels").contains("(may cost money)"), "the select marks the others");
}

#[test]
fn cgemini_is_typed_in_a_new_terminal() {
    let help = asset("/help.html").unwrap().0;
    let spec = include_str!("../docs/specs/gemini.md");
    for (name, text) in [("card", card_js()), ("help", help), ("spec", spec)] {
        assert!(text.contains("in a new terminal"), "{name} says a new terminal");
        assert!(!text.contains("in any terminal"), "{name}: a terminal open before has the old PATH");
    }
}
