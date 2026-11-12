// This computer's local clock for a moment: its hour and its day (yyyymmdd), for things
// done at a set time of night (docs/specs/desktop-renew.md).

pub(crate) struct Clock {
    pub(crate) ms: i64,
    pub(crate) hour: u32,
    pub(crate) day: i64,
}

impl Clock {
    #[cfg(windows)]
    pub(crate) fn at(ms: i64) -> Clock {
        use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
        use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
        let ft = (ms.max(0) as u64 + 11_644_473_600_000) * 10_000;
        let ft = FILETIME { dwLowDateTime: ft as u32, dwHighDateTime: (ft >> 32) as u32 };
        let loc = unsafe {
            let (mut utc, mut loc): (SYSTEMTIME, SYSTEMTIME) = (std::mem::zeroed(), std::mem::zeroed());
            FileTimeToSystemTime(&ft, &mut utc);
            SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut loc);
            loc
        };
        Clock { ms, hour: loc.wHour as u32, day: loc.wYear as i64 * 10_000 + loc.wMonth as i64 * 100 + loc.wDay as i64 }
    }

    // Elsewhere: UTC (the renewal is a Windows feature).
    #[cfg(not(windows))]
    pub(crate) fn at(ms: i64) -> Clock {
        Clock { ms, hour: ((ms / 3_600_000) % 24) as u32, day: ms / 86_400_000 }
    }
}
