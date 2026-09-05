use super::*;
use std::sync::Arc;

#[derive(Default)]
struct Memory {
    encoded: Option<String>,
    reads: usize,
    writes: usize,
    fail_read: bool,
    fail_write: bool,
}

#[derive(Clone, Default)]
struct TestBackend(Arc<Mutex<Memory>>);

impl Backend for TestBackend {
    fn read(&self, account: &str) -> Result<Stored<String>, CredentialError> {
        assert_eq!(account, ACCOUNT);
        let mut memory = self.0.lock().unwrap();
        memory.reads += 1;
        if memory.fail_read {
            return Err(CredentialError::StoreSynchronization);
        }
        Ok(match &memory.encoded {
            Some(value) => Stored::Ready(value.clone()),
            None => Stored::Missing,
        })
    }

    fn save(&self, value: &str) -> Result<(), CredentialError> {
        let mut memory = self.0.lock().unwrap();
        if memory.fail_write {
            return Err(CredentialError::StoreSynchronization);
        }
        memory.writes += 1;
        memory.encoded = Some(value.to_owned());
        Ok(())
    }
}

struct Fixture {
    directory: std::path::PathBuf,
    backend: TestBackend,
}

impl Fixture {
    fn new() -> Self {
        Self {
            directory: std::env::temp_dir()
                .join(format!("vesper-credentials-{}", uuid::Uuid::new_v4())),
            backend: TestBackend::default(),
        }
    }

    fn store(&self) -> CredentialStore<TestBackend> {
        CredentialStore {
            backend: self.backend.clone(),
            cache: None,
        }
    }

    fn path(&self) -> std::path::PathBuf {
        self.directory.join("credentials.lock")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn reads_the_keychain_once_for_multiple_providers() {
    let fixture = Fixture::new();
    fixture.backend.0.lock().unwrap().encoded =
        Some(r#"{"entries":{"my-memos-api":"memo-token","cloudflare-r2":"r2-token"}}"#.to_owned());
    let mut store = fixture.store();
    assert!(
        matches!(store.read(&fixture.path(), "my-memos-api").unwrap(), Stored::Ready(value) if value == "memo-token")
    );
    assert!(
        matches!(store.read(&fixture.path(), "cloudflare-r2").unwrap(), Stored::Ready(value) if value == "r2-token")
    );
    assert!(matches!(
        store.read(&fixture.path(), "ntfy-notifications").unwrap(),
        Stored::Missing
    ));
    assert_eq!(fixture.backend.0.lock().unwrap().reads, 1);
    assert_eq!(std::fs::read(fixture.path()).unwrap().len(), 16);
}

#[test]
fn missing_credentials_do_not_write_a_keychain_entry() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    assert!(matches!(
        store.read(&fixture.path(), "my-memos-api").unwrap(),
        Stored::Missing
    ));
    assert!(matches!(
        store.read(&fixture.path(), "cloudflare-r2").unwrap(),
        Stored::Missing
    ));
    let memory = fixture.backend.0.lock().unwrap();
    assert_eq!(memory.reads, 1);
    assert_eq!(memory.writes, 0);
}

#[test]
fn coordinates_cached_readers_and_preserves_other_provider_updates() {
    let fixture = Fixture::new();
    let mut desktop = fixture.store();
    let mut cli = fixture.store();
    desktop.read(&fixture.path(), "my-memos-api").unwrap();
    cli.read(&fixture.path(), "my-memos-api").unwrap();
    desktop
        .save(&fixture.path(), "my-memos-api", Some("memo-token"))
        .unwrap();
    cli.save(&fixture.path(), "cloudflare-r2", Some("r2-token"))
        .unwrap();
    assert!(
        matches!(desktop.read(&fixture.path(), "cloudflare-r2").unwrap(), Stored::Ready(value) if value == "r2-token")
    );
    assert!(
        matches!(cli.read(&fixture.path(), "my-memos-api").unwrap(), Stored::Ready(value) if value == "memo-token")
    );
    assert_eq!(fixture.backend.0.lock().unwrap().reads, 4);
}

#[test]
fn failed_writes_do_not_replace_cached_credentials() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    store
        .save(&fixture.path(), "app-lock", Some("original"))
        .unwrap();
    fixture.backend.0.lock().unwrap().fail_write = true;
    assert!(
        store
            .save(&fixture.path(), "app-lock", Some("replacement"))
            .is_err()
    );
    assert!(
        matches!(store.read(&fixture.path(), "app-lock").unwrap(), Stored::Ready(value) if value == "original")
    );
}

#[test]
fn rejects_unreadable_or_invalid_credentials_without_overwriting_them() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    fixture.backend.0.lock().unwrap().fail_read = true;
    assert!(
        store
            .save(&fixture.path(), "app-lock", Some("new-password"))
            .is_err()
    );
    fixture.backend.0.lock().unwrap().fail_read = false;
    fixture.backend.0.lock().unwrap().encoded = Some(r#""private-value""#.to_owned());
    assert!(matches!(
        store.read(&fixture.path(), "app-lock"),
        Err(CredentialError::InvalidStore)
    ));
    assert_eq!(fixture.backend.0.lock().unwrap().writes, 0);
}

#[test]
fn deletion_survives_restart_and_preserves_other_credentials() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    store
        .save(&fixture.path(), "app-lock", Some("password"))
        .unwrap();
    store
        .save(&fixture.path(), "my-memos-api", Some("token"))
        .unwrap();
    store.save(&fixture.path(), "app-lock", None).unwrap();
    let mut restarted = fixture.store();
    assert!(matches!(
        restarted.read(&fixture.path(), "app-lock").unwrap(),
        Stored::Missing
    ));
    assert!(
        matches!(restarted.read(&fixture.path(), "my-memos-api").unwrap(), Stored::Ready(value) if value == "token")
    );
}

#[test]
fn concurrent_writers_preserve_each_provider() {
    let fixture = Fixture::new();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    std::thread::scope(|scope| {
        for index in 0..8 {
            let mut store = fixture.store();
            let path = fixture.path();
            let barrier = barrier.clone();
            scope.spawn(move || {
                barrier.wait();
                store
                    .save(
                        &path,
                        &format!("provider-{index}"),
                        Some(&format!("value-{index}")),
                    )
                    .unwrap();
            });
        }
    });
    let mut reader = fixture.store();
    for index in 0..8 {
        assert!(
            matches!(reader.read(&fixture.path(), &format!("provider-{index}")).unwrap(), Stored::Ready(value) if value == format!("value-{index}"))
        );
    }
    assert_eq!(fixture.backend.0.lock().unwrap().writes, 8);
}
