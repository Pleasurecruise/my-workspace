use super::{CheckRequest, LoginRequest, encrypt_password};
use openssl::rsa::{Padding, Rsa};

#[test]
fn serializes_the_minimal_check_request() {
    let request = serde_json::to_value(CheckRequest { username: "admin" }).unwrap();

    assert_eq!(request, serde_json::json!({ "username": "admin" }));
}

#[test]
fn serializes_the_minimal_login_request() {
    let request = serde_json::to_value(LoginRequest {
        username: "admin",
        password: "encrypted".to_owned(),
        keepalive: true,
        otp: true,
        is_simple: true,
    })
    .unwrap();

    assert_eq!(
        request,
        serde_json::json!({
            "username": "admin",
            "password": "encrypted",
            "keepalive": true,
            "otp": true,
            "is_simple": true,
        })
    );
}

#[test]
fn encrypts_passwords_from_pkcs1_and_spki_public_keys() {
    let private_key = Rsa::generate(2048).unwrap();
    let pkcs1 = String::from_utf8(private_key.public_key_to_pem_pkcs1().unwrap()).unwrap();
    let spki = String::from_utf8(private_key.public_key_to_pem().unwrap()).unwrap();

    for public_key in [pkcs1, spki] {
        let encrypted = encrypt_password(&public_key, "secret-password").unwrap();
        let mut decrypted = vec![0; usize::try_from(private_key.size()).unwrap()];
        let decrypted_len = private_key
            .private_decrypt(&encrypted, &mut decrypted, Padding::PKCS1)
            .unwrap();

        assert_eq!(&decrypted[..decrypted_len], b"secret-password");
    }
}

#[test]
fn accepts_spki_keys_with_rsa_public_key_labels() {
    let private_key = Rsa::generate(2048).unwrap();
    let mislabeled = String::from_utf8(private_key.public_key_to_pem().unwrap())
        .unwrap()
        .replace("BEGIN PUBLIC KEY", "BEGIN RSA PUBLIC KEY")
        .replace("END PUBLIC KEY", "END RSA PUBLIC KEY");

    let encrypted = encrypt_password(&mislabeled, "secret-password").unwrap();
    let mut decrypted = vec![0; usize::try_from(private_key.size()).unwrap()];
    let decrypted_len = private_key
        .private_decrypt(&encrypted, &mut decrypted, Padding::PKCS1)
        .unwrap();

    assert_eq!(&decrypted[..decrypted_len], b"secret-password");
}
