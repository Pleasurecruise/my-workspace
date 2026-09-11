use super::*;

#[tokio::test]
async fn dated_expenses() {
    let directory = tempfile::tempdir().unwrap();
    let store = ledger::Store::new(directory.path().join("vesper.sqlite3"));
    let created = execute(
        &store,
        "2024-02-29",
        "create",
        &["12.34".into(), "Coffee shop".into(), "Oat latte".into()],
    )
    .await
    .unwrap();
    let id = created.entries[0].id.clone();
    assert_eq!(
        store.read("2024-02-29").await.unwrap().day_total_pence,
        1234
    );
    let edited = execute(
        &store,
        "2024-02-29",
        "update",
        &[id.clone(), "2.01".into(), "Dining".into()],
    )
    .await
    .unwrap();
    assert_eq!(edited.entries[0].category, "Dining");
    assert_eq!(edited.entries[0].description.as_deref(), Some("Oat latte"));
    let cleared = execute(
        &store,
        "2024-02-29",
        "update",
        &[id.clone(), "2.01".into(), "Dining".into(), "".into()],
    )
    .await
    .unwrap();
    assert_eq!(cleared.entries[0].description, None);
    let month = execute(&store, "2024-02-01", "list", &[]).await.unwrap();
    assert!(month.entries.is_empty());
    assert_eq!(month.month_total_pence, 201);
    assert_eq!(month.days.len(), 29);
    assert!(
        execute(&store, "2024-03-01", "delete", std::slice::from_ref(&id))
            .await
            .is_err()
    );
    let deleted = execute(&store, "2024-02-29", "delete", &[id])
        .await
        .unwrap();
    assert!(deleted.entries.is_empty());
    assert_eq!(deleted.month_total_pence, 0);
}

#[tokio::test]
async fn invalid_arguments() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("vesper.sqlite3");
    let store = ledger::Store::new(path.clone());
    for (date, action, arguments) in [
        ("2024-02-29", "create", vec!["1".into()]),
        ("2024-02-29", "list", vec!["extra".into()]),
        ("2024-02-30", "list", vec![]),
        (
            "2024-02-29",
            "create",
            vec!["1.001".into(), "Dining".into()],
        ),
    ] {
        assert!(execute(&store, date, action, &arguments).await.is_err());
        assert!(!path.exists());
    }
}
