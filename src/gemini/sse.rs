// Our streamed answer to Claude Code, written straight to the connection.
// Why not tiny_http's Response with a reader: it copies the reader through
// chunked_transfer's Encoder, which keeps 8 KB before it writes a chunk, so short SSE
// events would wait (measured in sse_tests.rs). Here each event is its own chunk,
// flushed at once; the last chunk ends the answer.

use std::io::{self, Write};

// HTTP/1.1: chunks. HTTP/1.0 has none: the body ends when the connection does (tiny_http
// closes it after a 1.0 request).
pub(crate) fn head(chunked: bool) -> String {
    let mut h = String::from("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n");
    if chunked {
        h.push_str("Transfer-Encoding: chunked\r\n");
    }
    h + "\r\n"
}

// One chunk; nothing for empty text (a zero-length chunk is the end of the answer).
pub(crate) fn frame(text: &str, chunked: bool) -> Vec<u8> {
    if text.is_empty() {
        return vec![];
    }
    if !chunked {
        return text.as_bytes().to_vec();
    }
    let mut out = format!("{:x}\r\n", text.len()).into_bytes();
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(b"\r\n");
    out
}

pub(crate) struct Sse {
    w: Box<dyn Write + Send>,
    chunked: bool,
}

impl Sse {
    // Status and headers, sent now: Claude Code knows the answer has begun.
    pub(crate) fn open(rq: tiny_http::Request) -> io::Result<Sse> {
        let chunked = *rq.http_version() >= (1, 1);
        let mut w = rq.into_writer();
        w.write_all(head(chunked).as_bytes())?;
        w.flush()?;
        Ok(Sse { w, chunked })
    }

    pub(crate) fn send(&mut self, text: &str) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        self.w.write_all(&frame(text, self.chunked))?;
        self.w.flush()
    }

    // The last chunk. Dropping the writer then lets tiny_http go on with the connection:
    // it closes it as Claude Code does after `Connection: close`.
    pub(crate) fn end(mut self) {
        if self.chunked {
            let _ = self.w.write_all(b"0\r\n\r\n");
        }
        let _ = self.w.flush();
    }
}
