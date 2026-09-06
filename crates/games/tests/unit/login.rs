use super::*;

#[tokio::test]
async fn cancelling_an_old_qr_does_not_remove_the_new_login() {
    let logins = Logins::default();
    logins.0.lock().unwrap().insert(
        Provider::Mihoyo,
        Slot {
            id: "new".into(),
            expires_at: transport::now() + 120,
            pending: None,
        },
    );
    logins.cancel(Provider::Mihoyo, "old").await;
    assert!(logins.0.lock().unwrap().contains_key(&Provider::Mihoyo));
    assert!(logins.poll(Provider::Mihoyo, "old").await.is_err());
    logins.cancel(Provider::Mihoyo, "new").await;
    assert!(logins.poll(Provider::Mihoyo, "new").await.is_err());
}

#[tokio::test]
async fn expired_qr_never_contacts_the_provider() {
    let logins = Logins::default();
    logins.0.lock().unwrap().insert(
        Provider::Skland,
        Slot {
            id: "expired".into(),
            expires_at: transport::now() - 1,
            pending: Some(Pending {
                ticket: "unused".into(),
                device: String::new(),
            }),
        },
    );
    assert!(matches!(
        logins.poll(Provider::Skland, "expired").await.unwrap(),
        LoginProgress::Expired
    ));
}
