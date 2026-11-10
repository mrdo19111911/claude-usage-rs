// AES-256-GCM decrypt via Windows CNG (BCrypt). Opens Claude Desktop's safeStorage
// values. Returns None on any failure, a tag mismatch included. Read only.

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::core::PCWSTR;
use windows_sys::Win32::Security::Cryptography::{
    BCryptCloseAlgorithmProvider, BCryptDecrypt, BCryptDestroyKey, BCryptGenerateSymmetricKey,
    BCryptOpenAlgorithmProvider, BCryptSetProperty, BCRYPT_AES_ALGORITHM,
    BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO, BCRYPT_CHAINING_MODE, BCRYPT_CHAIN_MODE_GCM,
};

// Byte length (with the null) of a wide-string constant, for BCryptSetProperty.
unsafe fn wbytes(p: PCWSTR) -> u32 {
    let mut n = 0usize;
    while *p.add(n) != 0 {
        n += 1;
    }
    ((n + 1) * 2) as u32
}

pub(super) fn decrypt(key: &[u8], nonce: &[u8], ct: &[u8], tag: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let mut alg: *mut c_void = null_mut();
        if BCryptOpenAlgorithmProvider(&mut alg, BCRYPT_AES_ALGORITHM, null(), 0) != 0 {
            return None;
        }
        let out = decrypt_with(alg, key, nonce, ct, tag);
        BCryptCloseAlgorithmProvider(alg, 0);
        out
    }
}

unsafe fn decrypt_with(alg: *mut c_void, key: &[u8], nonce: &[u8], ct: &[u8], tag: &[u8]) -> Option<Vec<u8>> {
    if BCryptSetProperty(alg, BCRYPT_CHAINING_MODE, BCRYPT_CHAIN_MODE_GCM as *const u8, wbytes(BCRYPT_CHAIN_MODE_GCM), 0) != 0 {
        return None;
    }
    let mut hkey: *mut c_void = null_mut();
    if BCryptGenerateSymmetricKey(alg, &mut hkey, null_mut(), 0, key.as_ptr(), key.len() as u32, 0) != 0 {
        return None;
    }
    let mut info: BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO>() as u32;
    info.dwInfoVersion = 1; // BCRYPT_INIT_AUTH_MODE_INFO_VERSION
    info.pbNonce = nonce.as_ptr() as *mut u8;
    info.cbNonce = nonce.len() as u32;
    info.pbTag = tag.as_ptr() as *mut u8;
    info.cbTag = tag.len() as u32;
    let mut buf = vec![0u8; ct.len()];
    let mut done = 0u32;
    let st = BCryptDecrypt(hkey, ct.as_ptr(), ct.len() as u32, &info as *const _ as *const c_void, null_mut(), 0, buf.as_mut_ptr(), buf.len() as u32, &mut done, 0);
    BCryptDestroyKey(hkey);
    if st != 0 {
        return None;
    }
    buf.truncate(done as usize);
    Some(buf)
}
