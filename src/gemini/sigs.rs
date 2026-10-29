// Gemini 3's thought signatures, kept by the tool_use id they came with: Claude Code
// sends back only id, name and input, and Gemini refuses a function call without one.
// As agcp's signature_cache.rs: 2 hours, at most 1000, the oldest dropped first.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(2 * 60 * 60);
const MAX: usize = 1000;

// id → (signature, when kept, order kept): the order breaks ties between equal Instants.
type Kept = HashMap<String, (String, Instant, u64)>;

// The kept signatures and the last order number given.
#[derive(Default)]
pub(crate) struct Sigs(Mutex<(Kept, u64)>);

impl Sigs {
    pub(crate) fn put(&self, id: &str, sig: &str) {
        if id.is_empty() || sig.is_empty() {
            return;
        }
        let mut g = self.0.lock().unwrap();
        g.1 += 1;
        let seq = g.1;
        let m = &mut g.0;
        if m.len() >= MAX && !m.contains_key(id) {
            let now = Instant::now();
            m.retain(|_, (_, at, _)| now.duration_since(*at) <= TTL);
            // Still full: the oldest go, a tenth at a time so this sort is rare.
            if m.len() >= MAX {
                let mut by_age: Vec<(String, u64)> = m.iter().map(|(k, (_, _, n))| (k.clone(), *n)).collect();
                by_age.sort_by_key(|(_, n)| *n);
                for (k, _) in by_age.into_iter().take(m.len() + 1 - MAX + MAX / 10) {
                    m.remove(&k);
                }
            }
        }
        m.insert(id.to_string(), (sig.to_string(), Instant::now(), seq));
    }

    pub(crate) fn get(&self, id: &str) -> Option<String> {
        self.get_at(id, Instant::now())
    }

    // `now` is a parameter so the tests can look two hours ahead.
    fn get_at(&self, id: &str, now: Instant) -> Option<String> {
        let m = &mut self.0.lock().unwrap().0;
        let (sig, at, _) = m.get(id)?;
        if now.saturating_duration_since(*at) > TTL {
            m.remove(id);
            return None;
        }
        Some(sig.clone())
    }

}
