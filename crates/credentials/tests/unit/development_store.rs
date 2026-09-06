use super::*;

#[test]
fn persists_entries_and_deletions_across_restarts() {
    let directory = tempfile::tempdir().unwrap();
    {
        let mut store = Store::open(directory.path()).unwrap();
        assert!(store.entries.is_empty());
        store.save("app-lock", Some("test-password")).unwrap();
        store.save("ugos-certificate", Some("fingerprint")).unwrap();
    }
    {
        let mut store = Store::open(directory.path()).unwrap();
        assert_eq!(store.entries["app-lock"], "test-password");
        store.save("app-lock", None).unwrap();
    }
    let store = Store::open(directory.path()).unwrap();
    assert!(!store.entries.contains_key("app-lock"));
    assert_eq!(store.entries["ugos-certificate"], "fingerprint");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&store.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn preserves_corrupt_storage_without_falling_back() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("credentials.json");
    std::fs::write(&path, b"invalid").unwrap();
    assert!(Store::open(directory.path()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"invalid");
}

#[test]
fn concurrent_writers_preserve_other_accounts() {
    let directory = tempfile::tempdir().unwrap();
    std::thread::scope(|scope| {
        for account in [
            "app-lock",
            "telegram-publication",
            "x-publication",
            "ugos-certificate",
        ] {
            let path = directory.path();
            scope.spawn(move || {
                Store::open(path)
                    .unwrap()
                    .save(account, Some(account))
                    .unwrap()
            });
        }
    });
    let store = Store::open(directory.path()).unwrap();
    assert_eq!(store.entries.len(), 4);
}
