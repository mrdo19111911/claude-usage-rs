// Results of parallel calls in the order of the calls (docs/specs/gemini.md, "The
// proxy"): no ids go to Gemini, and it pairs same-name responses by position. CCR builds
// them by walking the calls the same way (docs/SO_SANH_GEMINI_PROXY.md, "Sửa sau review").

use crate::*;

// One piece of a turn: a part, or a tool_result's parts (its functionResponse, then its
// images) with the place of its call among all calls so far.
pub(super) enum Item {
    Part(Value),
    Result(usize, Vec<Value>),
}

impl Item {
    pub(super) fn is_call(&self) -> bool {
        matches!(self, Item::Part(p) if p.get("functionCall").is_some())
    }
}

// A turn's parts: the results sorted by their call's place, in the slots results held;
// every other part stays where it was.
pub(super) fn flatten(items: Vec<Item>) -> Vec<Value> {
    let mut results: Vec<(usize, Vec<Value>)> = vec![];
    // None: a result's slot.
    let mut slots: Vec<Option<Value>> = vec![];
    for i in items {
        match i {
            Item::Part(p) => slots.push(Some(p)),
            Item::Result(n, p) => {
                results.push((n, p));
                slots.push(None);
            }
        }
    }
    results.sort_by_key(|(n, _)| *n);
    let mut next = results.into_iter().map(|(_, p)| p);
    slots.into_iter().flat_map(|s| s.map_or_else(|| next.next().unwrap_or_default(), |p| vec![p])).collect()
}
