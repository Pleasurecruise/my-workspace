use super::CertFingerprint;

#[test]
fn parses_fingerprint() {
    let fingerprint = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(CertFingerprint::from_hex(fingerprint).is_ok());
}

#[test]
fn rejects_short_hash() {
    assert!(CertFingerprint::from_hex("01234567").is_err());
}
