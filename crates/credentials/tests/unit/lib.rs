use super::{
    ConsumerApi, CredentialError, NtfyConfig, R2Credentials, UgosCredentials, save_app_lock,
    save_consumer_api, save_ntfy, save_r2, save_ugos,
};

#[test]
fn rejects_empty_ugos_user() {
    let result = save_ugos(UgosCredentials {
        username: String::new(),
        password: "password".to_owned(),
    });
    assert!(matches!(
        result,
        Err(CredentialError::Empty("UGOS username"))
    ));
}

#[test]
fn rejects_empty_api_key() {
    assert!(matches!(
        save_consumer_api(ConsumerApi::Memos, "  "),
        Err(CredentialError::Empty("my-memos API key"))
    ));
}

#[test]
fn rejects_empty_r2_secret() {
    let result = save_r2(R2Credentials {
        access_key_id: "access".to_owned(),
        secret_access_key: String::new(),
    });
    assert!(matches!(
        result,
        Err(CredentialError::Empty("R2 secret access key"))
    ));
}

#[test]
fn rejects_short_lock() {
    assert!(matches!(
        save_app_lock("123"),
        Err(CredentialError::TooShort("app lock password", 4))
    ));
}

#[test]
fn rejects_empty_ntfy() {
    assert!(matches!(
        save_ntfy(NtfyConfig {
            token: "  ".to_owned(),
            development: false,
        }),
        Err(CredentialError::Empty("ntfy token"))
    ));
}
