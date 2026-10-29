// What a Google AI Studio key may be (docs/specs/gemini.md): one rule for a key pasted on
// the page and one copied in AI Studio (watch.rs), so whatever the watch takes can be saved.

// `AQ.` + 200 characters at most: the longest copied key (AI Studio's format since May 2026).
const AQ_MAX: usize = 200;
pub(crate) const MAX_LEN: usize = "AQ.".len() + AQ_MAX;

// A pasted key goes into a header: printable ASCII only, no spaces, MAX_LEN at most.
pub(crate) fn fits(key: &str) -> bool {
    !key.is_empty() && key.len() <= MAX_LEN && key.chars().all(|c| c.is_ascii_graphic())
}

// The key in the copied text, if the text (trimmed) is exactly one: `AIza` + 35 of
// [0-9A-Za-z_-], or `AQ.` + 20-200 of [0-9A-Za-z_.-] (AI Studio's format since May 2026).
pub(crate) fn key_in(text: &str) -> Option<String> {
    let t = text.trim();
    let all = |s: &str, extra: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c));
    let ok = match (t.strip_prefix("AIza"), t.strip_prefix("AQ.")) {
        (Some(rest), _) => rest.len() == 35 && all(rest, "_-"),
        (_, Some(rest)) => (20..=AQ_MAX).contains(&rest.len()) && all(rest, "_.-"),
        _ => false,
    };
    (ok && fits(t)).then(|| t.to_string())
}
