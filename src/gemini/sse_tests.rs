// Our streamed answer: the HTTP framing, and a measurement that each event leaves when
// it is sent (tiny_http's own chunked writer holds ~8 KB back; see sse.rs).

use super::sse::{frame, head, Sse};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn frames_chunks_and_never_an_empty_one() {
    assert_eq!(frame("abc", true), b"3\r\nabc\r\n");
    assert_eq!(frame(&"x".repeat(26), true)[..4], *b"1a\r\n");
    assert_eq!(frame("abc", false), b"abc");
    assert!(frame("", true).is_empty(), "an empty chunk would end the answer");
}

#[test]
fn head_says_event_stream() {
    let h = head(true);
    assert!(h.starts_with("HTTP/1.1 200 OK\r\n"));
    for l in ["Content-Type: text/event-stream\r\n", "Cache-Control: no-cache\r\n", "Connection: close\r\n", "Transfer-Encoding: chunked\r\n"] {
        assert!(h.contains(l), "{l}");
    }
    assert!(h.ends_with("\r\n\r\n"));
    assert!(!head(false).contains("Transfer-Encoding"), "HTTP/1.0 has no chunks");
}

// When (ms after the request) each of the events "e0".."e3" was first seen, and the
// whole answer once the server closed the connection.
fn measure(serve: impl FnOnce(tiny_http::Request) + Send + 'static) -> (Vec<u128>, String) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    thread::spawn(move || serve(server.recv().unwrap()));
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(format!("GET /x HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
    let t = Instant::now();
    let (mut all, mut seen, mut buf) = (String::new(), vec![], [0u8; 4096]);
    loop {
        let n = s.read(&mut buf).expect("the server closes the connection at the end");
        if n == 0 {
            break;
        }
        all.push_str(&String::from_utf8_lossy(&buf[..n]));
        while seen.len() < 4 && all.contains(&format!("e{}", seen.len())) {
            seen.push(t.elapsed().as_millis());
        }
    }
    (seen, all)
}

const GAP: u64 = 500;

#[test]
fn each_event_arrives_as_it_is_sent() {
    let (seen, all) = measure(|rq| {
        let mut out = Sse::open(rq).unwrap();
        for i in 0..4 {
            thread::sleep(Duration::from_millis(GAP));
            out.send(&format!("event: e{i}\ndata: {{}}\n\n")).unwrap();
        }
        out.end();
    });
    println!("Sse (into_writer, chunk + flush per event): events at {seen:?} ms");
    assert_eq!(seen.len(), 4, "{all}");
    for (i, ms) in seen.iter().enumerate() {
        let due = (i as u128 + 1) * GAP as u128;
        assert!(*ms >= due - 50 && *ms < due + 250, "event {i} at {ms} ms, sent at {due}");
    }
    assert!(all.ends_with("0\r\n\r\n"), "ends with the last chunk");
}

// For the record (cargo test -- --ignored --nocapture): the same events through
// tiny_http's Response with a reader. Not an assertion of ours, only what it does.
#[test]
#[ignore]
fn measure_tiny_http_reader_response() {
    struct Slow(u32);
    impl Read for Slow {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0 == 4 {
                return Ok(0);
            }
            thread::sleep(Duration::from_millis(GAP));
            let e = format!("event: e{}\ndata: {{}}\n\n", self.0);
            self.0 += 1;
            buf[..e.len()].copy_from_slice(e.as_bytes());
            Ok(e.len())
        }
    }
    let (seen, _) = measure(|rq| {
        let ctype = tiny_http::Header::from_bytes("Content-Type", "text/event-stream").unwrap();
        let _ = rq.respond(tiny_http::Response::new(200.into(), vec![ctype], Slow(0), None, None));
    });
    println!("tiny_http Response::new(reader): events at {seen:?} ms");
}
