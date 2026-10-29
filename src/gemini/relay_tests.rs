// Google's SSE stream read line by line and passed on, with pings while it is silent.

use super::relay::{relay, DataLines};
use std::io::{self, Read};
use std::time::{Duration, Instant};

#[test]
fn joins_data_lines_up_to_the_blank_line() {
    let mut d = DataLines::default();
    assert_eq!(d.feed("data: {\"a\":1}"), None);
    assert_eq!(d.feed(""), Some("{\"a\":1}".into()));
    assert_eq!(d.feed(""), None, "a lone blank line is nothing");
    assert_eq!(d.feed(": a comment"), None);
    assert_eq!(d.feed("event: x"), None);
    assert_eq!(d.feed("data:[1,"), None);
    assert_eq!(d.feed("data: 2]\r"), None);
    assert_eq!(d.feed("\r"), Some("[1,\n2]".into()));
    assert_eq!(d.feed("data: {}"), None);
    assert_eq!(d.end(), Some("{}".into()), "the last one without its blank line");
    assert_eq!(d.end(), None);
}

// A source that waits before each piece, as Google does between chunks.
struct Slow(Vec<(u64, &'static str)>);

impl Read for Slow {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.0.is_empty() {
            return Ok(0);
        }
        let (wait, text) = self.0.remove(0);
        std::thread::sleep(Duration::from_millis(wait));
        if text == "BROKEN" {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "timed out"));
        }
        buf[..text.len()].copy_from_slice(text.as_bytes());
        Ok(text.len())
    }
}

fn run(src: Slow, silence_ms: u64) -> (bool, Vec<String>) {
    let mut sent = vec![];
    let ok = relay(src, Duration::from_millis(silence_ms), |v| format!("got {}\n", v["n"]), |s| {
        sent.push(s.to_string());
        Ok(())
    })
    .unwrap();
    (ok, sent)
}

#[test]
fn passes_each_chunk_on_and_pings_while_silent() {
    let src = Slow(vec![(0, "data: {\"n\":1}\n\n"), (450, "data: {\"n\":2}\n\ndata: {\"n\""), (0, ":3}\n\n")]);
    let t = Instant::now();
    let (ok, sent) = run(src, 200);
    assert!(ok);
    assert!(t.elapsed() < Duration::from_millis(1500));
    let pings = sent.iter().filter(|s| s.contains("\"ping\"")).count();
    assert!((1..=3).contains(&pings), "pings: {sent:?}");
    let got: Vec<&String> = sent.iter().filter(|s| s.starts_with("got")).collect();
    assert_eq!(got, ["got 1\n", "got 2\n", "got 3\n"]);
}

#[test]
fn skips_what_is_not_json_and_what_translates_to_nothing() {
    let mut sent = vec![];
    let src = Slow(vec![(0, "data: nope\n\ndata: {\"n\":1}\n\ndata: {\"skip\":1}\n\n")]);
    let ok = relay(src, Duration::from_secs(5), |v| if v["skip"] == 1 { String::new() } else { "x".into() }, |s| {
        sent.push(s.to_string());
        Ok(())
    })
    .unwrap();
    assert!(ok);
    assert_eq!(sent, ["x"]);
}

#[test]
fn a_broken_stream_ends_with_an_error_event() {
    let (ok, sent) = run(Slow(vec![(0, "data: {\"n\":1}\n\n"), (0, "BROKEN")]), 5000);
    assert!(!ok);
    assert_eq!(sent.len(), 2);
    assert!(sent[1].starts_with("event: error\ndata: "), "{}", sent[1]);
    assert!(sent[1].contains("api_error"));
}

#[test]
fn stops_when_the_client_has_gone() {
    let src = Slow(vec![(0, "data: {\"n\":1}\n\n"), (0, "data: {\"n\":2}\n\n")]);
    let r = relay(src, Duration::from_secs(5), |_| "x".into(), |_| Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone")));
    assert!(r.is_err());
}
