use super::aes_gcm::decrypt;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

// NIST AES-256-GCM: 32-byte zero key, 12-byte zero IV, 16 zero plaintext bytes.
const CT: &str = "cea7403d4d606b6e074ec5d3baf39d18";
const TAG: &str = "d0d1c8a799996bf0265b98b5d48ab919";

#[test]
fn decrypts_a_known_answer_vector() {
    assert_eq!(decrypt(&[0u8; 32], &[0u8; 12], &hex(CT), &hex(TAG)), Some(vec![0u8; 16]));
}

#[test]
fn a_tampered_tag_fails() {
    let mut tag = hex(TAG);
    tag[0] ^= 1;
    assert_eq!(decrypt(&[0u8; 32], &[0u8; 12], &hex(CT), &tag), None);
}
