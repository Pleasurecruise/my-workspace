use super::{
    ConsumerApi, CredentialError, R2Credentials, UgosCredentials, save_app_lock, save_consumer_api,
    save_r2, save_ugos,
};

#[test]
fn rejects_an_empty_ugos_username_before_opening_the_keychain() {
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
fn rejects_empty_consumer_api_key_before_opening_the_store() {
    assert!(matches!(
        save_consumer_api(ConsumerApi::Memos, "  "),
        Err(CredentialError::Empty("my-memos API key"))
    ));
}

#[test]
fn rejects_an_empty_r2_secret_before_opening_the_keychain() {
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
fn rejects_a_short_app_lock_password_before_opening_the_keychain() {
    assert!(matches!(
        save_app_lock("123"),
        Err(CredentialError::TooShort("app lock password", 4))
    ));
}
