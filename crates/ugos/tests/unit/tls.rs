use super::CertFingerprint;

#[test]
fn parses_a_sha256_certificate_fingerprint() {
    let fingerprint = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(CertFingerprint::from_hex(fingerprint).is_ok());
}

#[test]
fn rejects_a_short_certificate_fingerprint() {
    assert!(CertFingerprint::from_hex("01234567").is_err());
}
