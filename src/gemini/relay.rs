// Google's SSE stream → ours: a thread reads Google's lines (it may wait up to the read
// timeout for each), this side passes each chunk on as it comes and sends a ping after
// `silence` with nothing, so Claude Code's idle watchdog does not cut the answer.

use crate::*;
use super::{errors, stream};
use std::io::{self, BufRead, BufReader, Read};
use std::sync::mpsc::{sync_channel, RecvTimeoutError};

// SSE lines → the `data:` payload of each event (several data lines joined by "\n").
#[derive(Default)]
pub(crate) struct DataLines {
    data: Vec<String>,
}

impl DataLines {
    pub(crate) fn feed(&mut self, line: &str) -> Option<String> {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            return self.end();
        }
        if let Some(d) = line.strip_prefix("data:") {
            self.data.push(d.strip_prefix(' ').unwrap_or(d).to_string());
        }
        None
    }

    // The event still open (at the end of the stream), if any.
    pub(crate) fn end(&mut self) -> Option<String> {
        if self.data.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.data).join("\n"))
    }
}

// Ok(true): Google's stream ended; the caller closes the message. Ok(false): it broke
// off; an error event has been sent instead. Err: Claude Code has gone (writing failed).
pub(crate) fn relay<R: Read + Send + 'static>(
    src: R,
    silence: Duration,
    mut chunk: impl FnMut(&Value) -> String,
    mut send: impl FnMut(&str) -> io::Result<()>,
) -> io::Result<bool> {
    // A few events of slack; when we stop reading (client gone), the thread's next send
    // fails and it ends, closing Google's connection with it.
    let (tx, rx) = sync_channel::<Result<String, String>>(16);
    thread::spawn(move || {
        let mut d = DataLines::default();
        for line in BufReader::new(src).lines() {
            let sent = match line {
                Ok(l) => match d.feed(&l) {
                    Some(p) => tx.send(Ok(p)),
                    None => Ok(()),
                },
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    return;
                }
            };
            if sent.is_err() {
                return;
            }
        }
        if let Some(p) = d.end() {
            let _ = tx.send(Ok(p));
        }
    });
    loop {
        match rx.recv_timeout(silence) {
            Ok(Ok(payload)) => {
                // A line that is not JSON (none expected) is skipped, not fatal.
                if let Ok(v) = serde_json::from_str::<Value>(&payload) {
                    let out = chunk(&v);
                    if !out.is_empty() {
                        send(&out)?;
                    }
                }
            }
            Ok(Err(e)) => {
                let err = errors::error_json("api_error", &format!("The connection to Gemini broke off: {e}"));
                send(&format!("event: error\ndata: {err}\n\n"))?;
                return Ok(false);
            }
            Err(RecvTimeoutError::Timeout) => send(&stream::ping())?,
            Err(RecvTimeoutError::Disconnected) => return Ok(true),
        }
    }
}
