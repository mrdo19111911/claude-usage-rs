// The Windows clipboard for "Copy to add" (watch.rs): its sequence number and its text.
// Read as clipboard-win's raw::get_string does (docs/SO_SANH_GEMINI_PROXY.md → Clipboard):
// OpenClipboard, GetClipboardData(CF_UNICODETEXT), GlobalLock, up to the first NUL. One
// try only: when another program has it open, the watcher tries on its next tick.
// Elsewhere than Windows there is nothing to read.

use super::watch::{Read, Source};

// Longer text cannot be a key (even with spaces around it), so it is not copied out.
pub(crate) const MAX_UNITS: usize = 1024;

pub(crate) struct System;

// UTF-16 up to the first NUL. None when longer than MAX_UNITS or not valid UTF-16.
pub(crate) fn text_of(units: &[u16]) -> Option<String> {
    let n = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    if n > MAX_UNITS {
        return None;
    }
    String::from_utf16(&units[..n]).ok()
}

#[cfg(windows)]
impl Source for System {
    fn seq(&self) -> u32 {
        unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() }
    }

    fn read(&self) -> Read {
        use windows_sys::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};
        // No window of ours: the clipboard is only read, so it needs no owner.
        if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
            return Read::Busy;
        }
        let r = unsafe { read_open() };
        unsafe { CloseClipboard() };
        r
    }
}

// The text, with the clipboard open.
#[cfg(windows)]
unsafe fn read_open() -> Read {
    use windows_sys::Win32::System::DataExchange::{GetClipboardData, IsClipboardFormatAvailable};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    let format = CF_UNICODETEXT as u32;
    if IsClipboardFormatAvailable(format) == 0 {
        return Read::Other;
    }
    let h = GetClipboardData(format);
    if h.is_null() {
        return Read::Other;
    }
    let p = GlobalLock(h) as *const u16;
    if p.is_null() {
        return Read::Other;
    }
    // One unit past the limit is enough to know the text is too long.
    let units = (GlobalSize(h) / 2).min(MAX_UNITS + 1);
    let text = text_of(std::slice::from_raw_parts(p, units));
    GlobalUnlock(h);
    text.map_or(Read::Other, Read::Text)
}

#[cfg(not(windows))]
impl Source for System {
    fn seq(&self) -> u32 {
        0
    }
    fn read(&self) -> Read {
        Read::Other
    }
}
