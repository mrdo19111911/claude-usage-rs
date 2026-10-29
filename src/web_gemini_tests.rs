// The "Claude Code on Gemini" card (docs/specs/gemini.md, "Onboarding" and "Copy to add"):
// the page has the card, its button and the three-step dialog; gemini.js and
// gemini_dialog.js are served, loaded and call the endpoints; the help page explains it.

use super::asset;

fn page() -> &'static str {
    asset("/").unwrap().0
}

// Both scripts of the card: the card itself and its dialog.
fn js() -> String {
    let card = asset("/js/gemini.js").expect("gemini.js served").0;
    let dlg = asset("/js/gemini_dialog.js").expect("gemini_dialog.js served").0;
    format!("{card}\n{dlg}")
}

fn dialog() -> &'static str {
    let p = page();
    let start = p.find("<dialog id=\"gemDlg\"").expect("a <dialog>");
    &p[start..start + p[start..].find("</dialog>").unwrap()]
}

#[test]
fn the_card_comes_after_the_claude_accounts() {
    let p = page();
    let card = p.find("id=\"gemCard\"").expect("a Gemini card");
    assert!(p.find("Claude accounts").unwrap() < card, "after Claude accounts");
    assert!(card < p.find("<h2>Settings</h2>").unwrap(), "before Settings");
    assert!(p.contains("Claude Code on Gemini"));
    assert!(p.contains("instead of <code>claude</code>"), "one line on what it is");
    assert!(p.contains("id=\"gemBody\""), "room for the ready state");
}

#[test]
fn the_add_button_has_its_text() {
    let p = page();
    let i = p.find("id=\"gemAdd\"").expect("the add button");
    let text = &p[i..i + p[i..].find("</button>").unwrap()];
    assert!(text.ends_with(">+ Add Gemini account (free)"), "{text}");
}

#[test]
fn the_dialog_has_three_steps_sign_in_copy_and_nothing_else() {
    let dlg = dialog();
    let ol = &dlg[dlg.find("<ol").unwrap()..dlg.find("</ol>").unwrap()];
    assert_eq!(ol.matches("<li>").count(), 3, "three steps");
    for w in ["Sign in with your Google account", "accept Google's terms", "<b>Copy</b>",
              "two small squares", "No code yet?", "Create API key", "picks it up by itself"] {
        assert!(ol.contains(w), "steps say {w}");
    }
    assert!(dlg.contains("access code"), "the key is called an access code");
    assert_eq!(dlg.matches("“API key”").count(), 1, "the words API key once, in the small text");
    assert!(dlg.contains("id=\"gemWatch\"") && dlg.contains("role=\"status\""), "a live status line");
    assert!(dlg.contains(">Open Google's page again</a>"), "a link to open the page again");
    assert!(dlg.contains("href=\"https://aistudio.google.com/app/apikey\""));
    assert!(!dlg.to_lowercase().contains("email"), "no email or password asked");
}

#[test]
fn the_paste_field_show_and_done_are_inside_or_paste_it_here() {
    let dlg = dialog();
    let start = dlg.find("<details").expect("a <details> for pasting");
    let det = &dlg[start..start + dlg[start..].find("</details>").unwrap()];
    assert!(det.contains("<summary>Or paste it here</summary>"));
    assert!(det.contains("<input type=\"password\" id=\"gemKey\""), "the key is hidden");
    assert!(det.contains("<label for=\"gemKey\""), "the field has a label");
    for id in ["gemShow", "gemDone", "gemErr"] {
        assert!(det.contains(&format!("id=\"{id}\"")), "{id} in the paste part");
    }
    assert!(det.contains(">Done</button>"));
    assert!(!det.contains("gemCancel") && dlg.contains(">Cancel</button>"), "Cancel stays outside");
}

#[test]
fn gemini_scripts_are_loaded_before_main_and_drawn_with_the_rest() {
    let p = page();
    for f in ["js/gemini.js", "js/gemini_dialog.js"] {
        assert!(p.contains(&format!("src=\"{f}\"")), "{f} loaded");
        assert!(p.find(f).unwrap() < p.find("js/main.js").unwrap(), "{f} before main.js");
        assert_eq!(asset(&format!("/{f}")).unwrap().1, "text/javascript; charset=utf-8");
    }
    assert!(js().contains("function renderGemini"));
    assert!(asset("/js/main.js").unwrap().0.contains("renderGemini()"), "drawn by render()");
}

#[test]
fn gemini_js_uses_the_endpoints_and_opens_ai_studio() {
    let j = js();
    for s in ["'/api/gemini/key'", "'/api/gemini/model'", "'/api/gemini', { method: 'DELETE' }",
              "'/api/gemini/watch'", "https://aistudio.google.com/app/apikey", "'_blank', 'noopener'",
              "showModal()"] {
        assert!(j.contains(s), "the scripts have {s}");
    }
    assert!(j.contains("S.gemini || null"), "a missing `gemini` is treated as null");
    assert!(j.contains("Checking the key"), "says it is checking while Google answers");
    assert!(j.contains("Your Gemini account is ready. Open a new terminal and type <code>${cmd}</code> to start coding."),
            "the success text");
    assert!(j.contains("g.command || 'cgemini'"), "the command, `cgemini` unless the app says otherwise");
}

#[test]
fn the_watch_starts_on_open_and_stops_on_close() {
    let j = js();
    assert!(j.contains("gemWatch(true)") && j.contains("gemWatch(false)"), "on and off");
    assert!(j.contains("JSON.stringify({ on })"), "the body is {{on}}");
    let open = &j[j.find("function openGemDlg").unwrap()..];
    let open = &open[..open.find("\n}").unwrap()];
    assert!(open.contains("gemWatch(true)"), "opening starts the watch");
    assert!(open.contains("setInterval(gemTick, 1e3)"), "and asks for the state every second");
    let stop = &j[j.find("function gemStop").unwrap()..];
    let stop = &stop[..stop.find("\n}").unwrap()];
    assert!(stop.contains("gemWatch(false)") && stop.contains("clearInterval(gemPoll)"), "closing stops both");
    assert!(j.contains("addEventListener('close', gemStop)") && j.contains("addEventListener('cancel', gemStop)"),
            "Done, Cancel, Esc and success all end in close or cancel");
    assert!(j.contains("< 60e3"), "the watch is renewed when under a minute is left");
}

#[test]
fn the_status_line_has_each_state() {
    let j = js();
    for w in ["S.geminiWatch || null", "'waiting'", "'checking'", "'error'", "Waiting for you to press <b>Copy</b>…",
              "Checking it with Google…", "Copy it again or try another", "esc(w.error"] {
        assert!(j.contains(w), "the status line has {w}");
    }
    assert!(j.contains("gemSaved = true; $('#gemDlg').close()"), "a saved key closes the dialog with the success text");
}

#[test]
fn the_key_stays_out_of_storage_logs_and_urls() {
    let j = js();
    for bad in ["localStorage", "sessionStorage", "console.", "?key=", "&key="] {
        assert!(!j.contains(bad), "the Gemini scripts must not use {bad}");
    }
    assert!(j.contains("$('#gemKey').value = ''"), "the field is cleared after");
    assert!(j.contains("function gemStop") && j.contains("gemClear()"), "closed any way clears it, Esc too");
}

#[test]
fn help_explains_cgemini() {
    let help = asset("/help.html").unwrap().0;
    let i = help.find("Claude Code on Gemini (cgemini)").expect("a help topic");
    let topic = &help[i..i + help[i..].find("</details>").unwrap()];
    for w in ["+ Add Gemini account (free)", "Create API key", "<b>Copy</b>", "Or paste it here",
              "only while that window is open", "looks exactly like a Google key",
              "Google sets", "state.json", "Remove", "cgemini"] {
        assert!(topic.contains(w), "the topic covers {w}");
    }
}
