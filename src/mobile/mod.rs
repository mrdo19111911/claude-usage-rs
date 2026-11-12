// A read-only feed of the dashboard's figures for a phone (docs/specs/mobile-feed.md).
// Off until main starts `serve::start`; nothing here touches Claude logins.
#![allow(dead_code)] // wired by the main session (docs/specs/mobile-feed.md, "Wiring")

pub(crate) mod feed;
pub(crate) mod pair;
pub(crate) mod peer;
pub(crate) mod serve;

/// The feed's port when none is set.
pub(crate) const DEFAULT_PORT: u16 = 4778;
