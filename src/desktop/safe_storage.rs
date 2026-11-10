// Claude Desktop's safeStorage on Windows: the DPAPI-wrapped app key in Local State,
// and opening a `v10` value with it (base64 of `v10` + 12-byte nonce + ciphertext +
// 16-byte GCM tag). Read only; nothing is written back.

use super::aes_gcm;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::Value;
use std::ffi::c_void;
use std::path::Path;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
use windows_sys::Win32::Foundation::LocalFree;

// DPAPI unwrap, for the current Windows user. None if it is not this user's or is corrupt.
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let in_blob = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        if CryptUnprotectData(&in_blob, null_mut(), null(), null(), null(), 0, &mut out) == 0 {
            return None;
        }
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut c_void);
        Some(v)
    }
}

// The app's AES key from Local State → os_crypt.encrypted_key (base64, "DPAPI"-prefixed).
pub(super) fn app_key(dir: &Path) -> Option<Vec<u8>> {
    let text = std::fs::read_to_string(dir.join("Local State")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let b64 = v.get("os_crypt")?.get("encrypted_key")?.as_str()?;
    let raw = STANDARD.decode(b64).ok()?;
    let stripped = raw.strip_prefix(b"DPAPI")?;
    unprotect(stripped)
}

// Decrypt a base64 `v10` value with the app key.
pub(super) fn decrypt_v10(key: &[u8], b64: &str) -> Option<Vec<u8>> {
    let raw = STANDARD.decode(b64).ok()?;
    let body = raw.strip_prefix(b"v10")?;
    if body.len() < 12 + 16 {
        return None;
    }
    let (nonce, rest) = body.split_at(12);
    let (ct, tag) = rest.split_at(rest.len() - 16);
    aes_gcm::decrypt(key, nonce, ct, tag)
}
