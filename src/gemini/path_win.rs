// Windows: read and write the user's `Path` (HKCU\Environment) as rustup does: kept as
// REG_EXPAND_SZ (entries like %USERPROFILE%\bin stay unexpanded), the value deleted when
// nothing is left, then WM_SETTINGCHANGE "Environment" so Explorer hands new terminals
// the new value.

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// The key, closed again when dropped.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

// The value now: None when it is not a string (left alone then, as rustup does).
fn read(k: &Key, name: &[u16]) -> Result<Option<String>, String> {
    let (mut kind, mut size) = (0u32, 0u32);
    let r = unsafe { RegQueryValueExW(k.0, name.as_ptr(), std::ptr::null(), &mut kind, std::ptr::null_mut(), &mut size) };
    if r == ERROR_FILE_NOT_FOUND {
        return Ok(Some(String::new()));
    }
    if r != ERROR_SUCCESS {
        return Err(format!("could not read PATH (error {r})"));
    }
    if kind != REG_SZ && kind != REG_EXPAND_SZ {
        return Ok(None);
    }
    let mut buf = vec![0u16; (size as usize).div_ceil(2)];
    let r = unsafe { RegQueryValueExW(k.0, name.as_ptr(), std::ptr::null(), &mut kind, buf.as_mut_ptr().cast(), &mut size) };
    if r != ERROR_SUCCESS {
        return Err(format!("could not read PATH (error {r})"));
    }
    buf.truncate(size as usize / 2);
    while buf.last() == Some(&0) {
        buf.pop();
    }
    Ok(Some(String::from_utf16_lossy(&buf)))
}

pub(crate) fn change(edit: impl Fn(&str) -> String) -> Result<(), String> {
    let mut h: HKEY = std::ptr::null_mut();
    let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide("Environment").as_ptr(), 0, KEY_READ | KEY_WRITE, &mut h) };
    if r != ERROR_SUCCESS {
        return Err(format!("could not open the user's environment (error {r})"));
    }
    let k = Key(h);
    let name = wide("Path");
    let Some(old) = read(&k, &name)? else { return Ok(()) };
    let new = edit(&old);
    if new == old {
        return Ok(());
    }
    let r = if new.is_empty() {
        unsafe { RegDeleteValueW(k.0, name.as_ptr()) }
    } else {
        let data = wide(&new);
        unsafe { RegSetValueExW(k.0, name.as_ptr(), 0, REG_EXPAND_SZ, data.as_ptr().cast(), (data.len() * 2) as u32) }
    };
    if r != ERROR_SUCCESS {
        return Err(format!("could not change PATH (error {r})"));
    }
    drop(k);
    // 5 s at most, and a hung window does not hold us up (SMTO_ABORTIFHUNG).
    let env = wide("Environment");
    unsafe { SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, env.as_ptr() as isize, SMTO_ABORTIFHUNG, 5000, std::ptr::null_mut()) };
    Ok(())
}
