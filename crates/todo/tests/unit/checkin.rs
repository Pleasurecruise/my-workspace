use super::*;

#[tokio::test]
async fn canonical_dates() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.sqlite3");
    let store = Store::new(path.clone());
    for date in ["+9999-12-31", "+2024-02-29", "0000-01-01", "-0001-01-01"] {
        assert!(matches!(
            store.set_check_in("read", date, true).await,
            Err(Error::InvalidDate(_))
        ));
        assert!(matches!(
            store.read_check_ins(vec!["read".into()], date).await,
            Err(Error::InvalidDate(_))
        ));
        assert!(!path.exists());
    }
}

#[test]
fn streaks_and_undo() {
    let directory = tempfile::tempdir().unwrap();
    let mut connection = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    for date in ["2024-02-28", "2024-02-29", "2024-03-01"] {
        set(&mut connection, "read", date, true).unwrap();
    }
    set(&mut connection, "read", "2024-03-01", true).unwrap();
    set(&mut connection, "walk", "2024-03-02", true).unwrap();
    let pending = read(&mut connection, "read", "2024-03-02", "2024-03-02").unwrap();
    assert_eq!(
        (pending.completed, pending.streak, pending.total),
        (false, 3, 3)
    );
    assert_eq!(pending.days.len(), 28);
    assert_eq!(pending.days.last().unwrap().date, "2024-03-02");
    set(&mut connection, "read", "2024-03-02", true).unwrap();
    assert_eq!(
        read(&mut connection, "read", "2024-03-02", "2024-03-02")
            .unwrap()
            .streak,
        4
    );
    set(&mut connection, "read", "2024-03-02", false).unwrap();
    set(&mut connection, "read", "2024-03-02", false).unwrap();
    assert_eq!(
        read(&mut connection, "read", "2024-03-03", "2024-03-03")
            .unwrap()
            .streak,
        0
    );
    drop(connection);
    let mut reopened = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    assert_eq!(
        read(&mut reopened, "read", "2024-03-02", "2024-03-02")
            .unwrap()
            .total,
        3
    );
    assert_eq!(
        read(&mut reopened, "walk", "2024-03-02", "2024-03-02")
            .unwrap()
            .total,
        1
    );
}

#[tokio::test]
async fn historical_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.sqlite3");
    let first = Store::new(path.clone());
    let second = Store::new(path);
    let historical = first
        .set_check_in("history", "2000-01-01", true)
        .await
        .unwrap();
    assert!(historical.completed);
    assert!(historical.editable);
    first
        .set_check_in("history", "2000-01-01", false)
        .await
        .unwrap();
    assert!(
        !first
            .read_check_ins(vec!["history".into()], "2000-01-01")
            .await
            .unwrap()[0]
            .completed
    );
    assert!(matches!(
        first.set_check_in("read", "9999-01-01", true).await,
        Err(Error::FutureCheckIn)
    ));
    assert!(
        !first
            .read_check_ins(vec!["read".into()], "9999-01-01")
            .await
            .unwrap()[0]
            .editable
    );
    assert!(matches!(
        first
            .read_check_ins(vec!["../bad".into()], "2000-01-01")
            .await,
        Err(Error::InvalidCheckIn)
    ));
    let date = crate::current_date().unwrap();
    let (a, b) = tokio::join!(
        first.set_check_in("read", &date, true),
        second.set_check_in("read", &date, true)
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(
        first
            .read_check_ins(vec!["read".into()], &date)
            .await
            .unwrap()[0]
            .total,
        1
    );
}

#[test]
fn history_window() {
    let directory = tempfile::tempdir().unwrap();
    let mut connection = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    let date = parse_date("2026-09-10").unwrap();
    for offset in 0..40 {
        let day = (date - time::Duration::days(offset)).to_string();
        set(&mut connection, "read", &day, true).unwrap();
    }
    set(&mut connection, "read", "2026-09-11", true).unwrap();
    let snapshot = read(&mut connection, "read", "2026-09-10", "2026-09-10").unwrap();
    assert_eq!(
        (snapshot.total, snapshot.streak, snapshot.days.len()),
        (40, 40, 28)
    );
    assert!(snapshot.days.iter().all(|day| day.completed));
    assert_eq!(snapshot.days.first().unwrap().date, "2026-08-14");
    let empty = read(&mut connection, "walk", "2026-09-10", "2026-09-10").unwrap();
    assert_eq!((empty.total, empty.streak), (0, 0));
    assert!(empty.days.iter().all(|day| !day.completed));
}

#[tokio::test]
async fn dated_habits() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::new(directory.path().join("test.sqlite3"));
    store
        .set_check_in("read", "2024-02-28", true)
        .await
        .unwrap();
    store
        .set_check_in("walk", "2024-02-29", true)
        .await
        .unwrap();
    let ids = vec!["walk".into(), "read".into()];
    let first = store
        .read_check_ins(ids.clone(), "2024-02-28")
        .await
        .unwrap();
    assert_eq!(first[0].id, "walk");
    assert!(!first[0].completed);
    assert!(first[1].completed);
    let second = store.read_check_ins(ids, "2024-02-29").await.unwrap();
    assert!(second[0].completed);
    assert!(!second[1].completed);
    assert!(second.iter().all(|state| state.date == "2024-02-29"));
}

#[test]
fn future_projection() {
    let directory = tempfile::tempdir().unwrap();
    let mut connection = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    set(&mut connection, "read", "2026-09-10", true).unwrap();
    set(&mut connection, "read", "2026-09-11", true).unwrap();
    let future = read(&mut connection, "read", "2026-09-11", "2026-09-10").unwrap();
    assert!(!future.completed);
    assert!(!future.editable);
    assert_eq!(future.total, 1);
    assert!(!future.days.last().unwrap().completed);
    let current = read(&mut connection, "read", "2026-09-11", "2026-09-11").unwrap();
    assert!(current.completed);
    assert_eq!(current.total, 2);
}
