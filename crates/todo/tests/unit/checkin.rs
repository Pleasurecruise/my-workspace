use super::*;

#[test]
fn streaks_cross_months_and_leap_days_and_undo_is_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let mut connection = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    for date in ["2024-02-28", "2024-02-29", "2024-03-01"] {
        set(&mut connection, "read", date, true).unwrap();
    }
    set(&mut connection, "read", "2024-03-01", true).unwrap();
    set(&mut connection, "walk", "2024-03-02", true).unwrap();
    let pending = read(&mut connection, "read", "2024-03-02").unwrap();
    assert_eq!(
        (pending.completed, pending.streak, pending.total),
        (false, 3, 3)
    );
    assert_eq!(pending.days.len(), 28);
    assert_eq!(pending.days.last().unwrap().date, "2024-03-02");
    set(&mut connection, "read", "2024-03-02", true).unwrap();
    assert_eq!(
        read(&mut connection, "read", "2024-03-02").unwrap().streak,
        4
    );
    set(&mut connection, "read", "2024-03-02", false).unwrap();
    set(&mut connection, "read", "2024-03-02", false).unwrap();
    assert_eq!(
        read(&mut connection, "read", "2024-03-03").unwrap().streak,
        0
    );
    drop(connection);
    let mut reopened = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    assert_eq!(read(&mut reopened, "read", "2024-03-02").unwrap().total, 3);
    assert_eq!(read(&mut reopened, "walk", "2024-03-02").unwrap().total, 1);
}

#[tokio::test]
async fn rejects_stale_dates_and_concurrent_check_ins_do_not_duplicate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.sqlite3");
    let first = Store::new(path.clone());
    let second = Store::new(path);
    assert!(matches!(
        first.set_check_in("read", "2000-01-01", true).await,
        Err(Error::CheckInDateChanged)
    ));
    assert!(matches!(
        first.read_check_in("../bad").await,
        Err(Error::InvalidCheckIn)
    ));
    let date = crate::current_date().unwrap();
    let (a, b) = tokio::join!(
        first.set_check_in("read", &date, true),
        second.set_check_in("read", &date, true)
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(first.read_check_in("read").await.unwrap().total, 1);
}

#[test]
fn recent_history_is_bounded_but_totals_and_streaks_are_not() {
    let directory = tempfile::tempdir().unwrap();
    let mut connection = vesper_database::open(&directory.path().join("test.sqlite3")).unwrap();
    let date = parse_date("2026-09-10").unwrap();
    for offset in 0..40 {
        let day = (date - time::Duration::days(offset)).to_string();
        set(&mut connection, "read", &day, true).unwrap();
    }
    set(&mut connection, "read", "2026-09-11", true).unwrap();
    let snapshot = read(&mut connection, "read", "2026-09-10").unwrap();
    assert_eq!(
        (snapshot.total, snapshot.streak, snapshot.days.len()),
        (40, 40, 28)
    );
    assert!(snapshot.days.iter().all(|day| day.completed));
    assert_eq!(snapshot.days.first().unwrap().date, "2026-08-14");
    let empty = read(&mut connection, "walk", "2026-09-10").unwrap();
    assert_eq!((empty.total, empty.streak), (0, 0));
    assert!(empty.days.iter().all(|day| !day.completed));
}
