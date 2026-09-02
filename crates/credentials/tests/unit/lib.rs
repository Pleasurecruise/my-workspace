use super::{
    ConsumerApi, CredentialError, NtfyConfig, R2Credentials, TelegramCredentials, UgosCredentials,
    XCredentials, save_app_lock, save_consumer_api, save_ntfy, save_r2, save_telegram, save_ugos,
    save_x,
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

#[test]
fn rejects_incomplete_telegram_credentials() {
    let result = save_telegram(TelegramCredentials {
        api_id: 12345,
        api_hash: "0123456789abcdef0123456789abcdef".to_owned(),
        channel_username: "  ".to_owned(),
    });
    assert!(matches!(
        result,
        Err(CredentialError::Empty("Telegram channel username"))
    ));
}

#[test]
fn rejects_invalid_telegram_channel() {
    let result = save_telegram(TelegramCredentials {
        api_id: 12345,
        api_hash: "0123456789abcdef0123456789abcdef".to_owned(),
        channel_username: "@my channel".to_owned(),
    });
    assert!(matches!(
        result,
        Err(CredentialError::InvalidValue(
            "Telegram channel username",
            _
        ))
    ));
}

#[test]
fn rejects_empty_x_token() {
    let result = save_x(XCredentials {
        client_id: "client".to_owned(),
        access_token: "  ".to_owned(),
        refresh_token: "refresh".to_owned(),
        expires_at: 1,
    });
    assert!(matches!(
        result,
        Err(CredentialError::Empty("X OAuth access token"))
    ));
}
