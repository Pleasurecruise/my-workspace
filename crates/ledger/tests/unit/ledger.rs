use super::*;

#[test]
fn amount_precision() {
    for (input, expected) in [
        ("0.01", 1),
        ("0.10", 10),
        ("12.3", 1230),
        ("12", 1200),
        (" 12.34 ", 1234),
        ("999999.99", MAX_PENCE),
    ] {
        assert_eq!(parse_amount(input).unwrap(), expected);
    }
    for input in [
        "",
        "0",
        "0.00",
        "-2",
        "+2",
        "1e3",
        "NaN",
        "1.001",
        "1,000",
        "1.",
        "1.2.3",
        "1000000",
        "999999999999999999999",
    ] {
        assert!(matches!(parse_amount(input), Err(Error::Amount)), "{input}");
    }
}

#[tokio::test]
async fn monthly_totals() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite3");
    let store = Store::new(path.clone());
    store
        .create("2024-02-01", "0.10", " eating out ", None)
        .await
        .unwrap();
    store
        .create("2024-02-29", "0.20", "EATING OUT", None)
        .await
        .unwrap();
    store
        .create("2024-02-29", "1.05", "书籍", None)
        .await
        .unwrap();
    store
        .create("2024-03-01", "500.00", "Housing", None)
        .await
        .unwrap();
    let snapshot = Store::new(path).read("2024-02-29").await.unwrap();
    assert_eq!(snapshot.month, "2024-02");
    assert_eq!(snapshot.day_total_pence, 125);
    assert_eq!(snapshot.month_total_pence, 135);
    assert_eq!(snapshot.entries.len(), 2);
    assert_eq!(snapshot.days.len(), 29);
    assert_eq!(snapshot.days[0].amount_pence, 10);
    assert_eq!(snapshot.days[1].amount_pence, 0);
    assert_eq!(snapshot.days[28].amount_pence, 125);
    assert_eq!(
        snapshot
            .categories
            .iter()
            .find(|item| item.category == "Eating out")
            .unwrap()
            .amount_pence,
        30
    );
    assert!(snapshot.suggestions.iter().any(|name| name == "书籍"));
    assert_eq!(store.read("2025-02-01").await.unwrap().days.len(), 28);
    assert_eq!(store.read("2024-12-31").await.unwrap().days.len(), 31);
}

#[tokio::test]
async fn dated_mutations() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("ledger.sqlite3"));
    let created = store
        .create("2026-09-11", "12.34", "Transport", None)
        .await
        .unwrap();
    let id = &created.entries[0].id;
    assert!(matches!(
        store
            .update("2026-09-12", id, "4.56", "Eating out", None)
            .await,
        Err(Error::MissingEntry)
    ));
    assert!(matches!(
        store.delete("2026-09-12", id).await,
        Err(Error::MissingEntry)
    ));
    let edited = store
        .update("2026-09-11", id, "4.56", "Eating out", None)
        .await
        .unwrap();
    assert_eq!(edited.month_total_pence, 456);
    assert_eq!(edited.entries[0].category, "Eating out");
    assert_eq!(edited.categories.len(), 1);
    let deleted = store.delete("2026-09-11", id).await.unwrap();
    assert_eq!(deleted.month_total_pence, 0);
    assert!(deleted.entries.is_empty());
    assert!(deleted.categories.is_empty());
    assert!(deleted.days.iter().all(|day| day.amount_pence == 0));
}

#[tokio::test]
async fn atomic_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite3");
    let store = Store::new(path.clone());
    let other = Store::new(path);
    for (date, amount, category) in [
        ("2026-02-30", "1", "Eating out"),
        ("2026-09-11", "-1", "Eating out"),
        ("2026-09-11", "1", " \n"),
        ("2026-09-11", "1", "Bad\tCategory"),
    ] {
        assert!(store.create(date, amount, category, None).await.is_err());
    }
    assert_eq!(store.read("2026-09-11").await.unwrap().month_total_pence, 0);
    let (a, b) = tokio::join!(
        store.create("2026-09-11", "1.11", "Coffee", None),
        other.create("2026-09-12", "2.22", "coffee", None)
    );
    a.unwrap();
    b.unwrap();
    let snapshot = store.read("2026-09-11").await.unwrap();
    assert_eq!(snapshot.month_total_pence, 333);
    assert_eq!(snapshot.categories.len(), 1);
    assert_eq!(snapshot.entries.len(), 1);
}

#[tokio::test]
async fn notes_survive_reopening_and_invalid_edits_are_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite3");
    let store = Store::new(path.clone());
    let created = store
        .create("2026-09-11", "3.50", "coffee", Some("  Oat latte  "))
        .await
        .unwrap();
    let entry = &created.entries[0];
    assert_eq!(entry.category, "Coffee");
    assert_eq!(entry.description.as_deref(), Some("Oat latte"));
    for invalid in ["x".repeat(501), "bad\0note".into(), "bad\nnote".into()] {
        assert!(matches!(
            store
                .update(&entry.date, &entry.id, "9", "Shopping", Some(&invalid))
                .await,
            Err(Error::Description)
        ));
    }
    let reopened = Store::new(path).read(&entry.date).await.unwrap();
    assert_eq!(
        reopened.entries[0].description.as_deref(),
        Some("Oat latte")
    );
    assert_eq!(reopened.day_total_pence, 350);
    let cleared = store
        .update(&entry.date, &entry.id, "3.50", "Coffee", Some("  "))
        .await
        .unwrap();
    assert_eq!(cleared.entries[0].description, None);
}

#[tokio::test]
async fn rejects_old_schema() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite3");
    let mut connection = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    connection.batch_execute("CREATE TABLE ledger_entries (id TEXT PRIMARY KEY NOT NULL, date TEXT NOT NULL, amount_pence BIGINT NOT NULL, category TEXT NOT NULL, created_at BIGINT NOT NULL); INSERT INTO ledger_entries VALUES ('old', '2026-09-11', 1234, 'Dining', 0);").unwrap();
    assert!(Store::new(path).read("2026-09-11").await.is_err());
    #[derive(QueryableByName)]
    struct Record {
        #[diesel(sql_type = diesel::sql_types::Text)]
        category: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        amount_pence: i64,
    }
    let record =
        diesel::sql_query("SELECT category, amount_pence FROM ledger_entries WHERE id = 'old'")
            .get_result::<Record>(&mut connection)
            .unwrap();
    assert_eq!(record.category, "Dining");
    assert_eq!(record.amount_pence, 1234);
}

#[tokio::test]
async fn suggestions_keep_other_last_after_custom_categories() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("ledger.sqlite3"));
    let snapshot = store
        .create("2026-09-11", "1", "Zebra", None)
        .await
        .unwrap();
    assert_eq!(
        snapshot.suggestions,
        [
            "Coffee",
            "Eating out",
            "Groceries",
            "Meal deal",
            "Shopping",
            "Subscriptions",
            "Transport",
            "Zebra",
            "Other"
        ]
    );
}
