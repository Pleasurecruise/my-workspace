use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "vesper.sqlite3";
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not access the application database: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not open the application database: {0}")]
    Connection(#[from] diesel::ConnectionError),
    #[error("Application database operation failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("Application database path is unavailable")]
    Path,
    #[error("Application database has an invalid SQLite header; existing data was preserved")]
    Header,
}

pub fn shared_path() -> Result<PathBuf, Error> {
    dirs::data_local_dir()
        .map(|directory| directory.join("me.you-find.vesper").join(FILE_NAME))
        .ok_or(Error::Path)
}

/// Opens the application-owned database. Feature owners use Diesel queries and
/// transactions on this connection; old feature files are never inspected.
pub fn open(path: &Path) -> Result<SqliteConnection, Error> {
    let parent = path.parent().ok_or(Error::Path)?;
    std::fs::create_dir_all(parent)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    if file.metadata()?.len() != 0 {
        use std::io::Read;
        let mut header = [0; 16];
        if file.read_exact(&mut header).is_err() || &header != b"SQLite format 3\0" {
            return Err(Error::Header);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    drop(file);
    let mut connection = SqliteConnection::establish(path.to_str().ok_or(Error::Path)?)?;
    connection.batch_execute(
        "PRAGMA busy_timeout = 5000; PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;",
    )?;
    connection.batch_execute(include_str!("schema.sql"))?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_during_a_write() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        let mut writer = open(&path).unwrap();
        writer.batch_execute("BEGIN IMMEDIATE;").unwrap();
        let reader = open(&path);
        writer.batch_execute("ROLLBACK;").unwrap();
        assert!(
            reader.is_ok(),
            "opening an initialized database needs no write lock"
        );
    }

    #[test]
    fn reopening_preserves_records_and_enforces_layout_constraints() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        open(&path).unwrap().batch_execute("INSERT INTO dashboard_widgets VALUES ('todo', 0, '{}'); INSERT INTO dashboard_layout VALUES (1, 'todo');").unwrap();
        let mut connection = open(&path).unwrap();
        assert!(
            connection
                .batch_execute("INSERT INTO dashboard_widgets VALUES ('todo', 0, '{}')")
                .is_err()
        );
        for invalid in [
            "INSERT INTO dashboard_widgets VALUES ('bad', -1, '{}')",
            "INSERT INTO dashboard_layout VALUES (2, NULL)",
            "UPDATE dashboard_layout SET island_widget_id = 'missing'",
        ] {
            assert!(connection.batch_execute(invalid).is_err());
        }
    }

    #[test]
    fn preserves_corrupt_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        std::fs::write(&path, b"broken").unwrap();
        assert!(matches!(open(&path), Err(Error::Header)));
        assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    }

    #[test]
    fn concurrent_initialization() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        let barrier = std::sync::Barrier::new(4);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    barrier.wait();
                    open(&path).unwrap();
                });
            }
        });
    }
}
