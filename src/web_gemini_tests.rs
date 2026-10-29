// The "Claude Code on Gemini" card (docs/specs/gemini.md, "Onboarding"): the page has the
// card, its button and the three-step dialog; gemini.js is served, loaded and calls the
// three endpoints; the help page explains it.

use super::asset;

fn page() -> &'static str {
    asset("/").unwrap().0
}

fn js() -> &'static str {
    asset("/js/gemini.js").expect("gemini.js served").0
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
fn the_dialog_has_three_steps_a_key_field_and_two_buttons() {
    let p = page();
    let start = p.find("<dialog id=\"gemDlg\"").expect("a <dialog>");
    let dlg = &p[start..start + p[start..].find("</dialog>").unwrap()];
    let ol = &dlg[dlg.find("<ol").unwrap()..dlg.find("</ol>").unwrap()];
    assert_eq!(ol.matches("<li>").count(), 3, "three steps");
    for w in ["Sign in with your Google account", "Create API key", "paste it here"] {
        assert!(ol.contains(w), "steps say {w}");
    }
    assert!(dlg.contains("<input type=\"password\" id=\"gemKey\""), "the key is hidden");
    assert!(dlg.contains("<label for=\"gemKey\""), "the field has a label");
    for id in ["gemShow", "gemDone", "gemCancel", "gemErr", "gemOpen"] {
        assert!(dlg.contains(&format!("id=\"{id}\"")), "{id} in the dialog");
    }
    assert!(dlg.contains(">Done</button>") && dlg.contains(">Cancel</button>"));
    assert!(dlg.contains("href=\"https://aistudio.google.com/app/apikey\""), "a link to open the page again");
    assert!(!dlg.to_lowercase().contains("email"), "no email or password asked");
}

#[test]
fn gemini_js_is_loaded_before_main_and_drawn_with_the_rest() {
    let p = page();
    assert!(p.contains("src=\"js/gemini.js\""), "gemini.js loaded");
    assert!(p.find("js/gemini.js").unwrap() < p.find("js/main.js").unwrap(), "before main.js");
    assert_eq!(asset("/js/gemini.js").unwrap().1, "text/javascript; charset=utf-8");
    assert!(js().contains("function renderGemini"));
    assert!(asset("/js/main.js").unwrap().0.contains("renderGemini()"), "drawn by render()");
}

#[test]
fn gemini_js_uses_the_three_endpoints_and_opens_ai_studio() {
    let j = js();
    for s in ["'/api/gemini/key'", "'/api/gemini/model'", "'/api/gemini', { method: 'DELETE' }",
              "https://aistudio.google.com/app/apikey", "'_blank', 'noopener'", "showModal()"] {
        assert!(j.contains(s), "gemini.js has {s}");
    }
    assert!(j.contains("S.gemini || null"), "a missing `gemini` is treated as null");
    assert!(j.contains("Checking the key"), "says it is checking while Google answers");
    assert!(j.contains("Your Gemini account is ready. Open a new terminal and type <code>${cmd}</code> to start coding."),
            "the success text");
    assert!(j.contains("g.command || 'cgemini'"), "the command, `cgemini` unless the app says otherwise");
}

#[test]
fn the_key_stays_out_of_storage_logs_and_urls() {
    let j = js();
    for bad in ["localStorage", "sessionStorage", "console.", "?key=", "&key="] {
        assert!(!j.contains(bad), "gemini.js must not use {bad}");
    }
    assert!(j.contains("$('#gemKey').value = ''"), "the field is cleared after");
    assert!(j.contains("addEventListener('cancel', gemClear)"), "Esc clears it too");
}

#[test]
fn help_explains_cgemini() {
    let help = asset("/help.html").unwrap().0;
    let i = help.find("Claude Code on Gemini (cgemini)").expect("a help topic");
    let topic = &help[i..i + help[i..].find("</details>").unwrap()];
    for w in ["+ Add Gemini account (free)", "Create API key", "Google sets", "state.json", "Remove", "cgemini"] {
        assert!(topic.contains(w), "the topic covers {w}");
    }
}
