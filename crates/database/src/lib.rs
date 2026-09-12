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
    if !ledger_has_description(&mut connection)? {
        connection.immediate_transaction::<_, diesel::result::Error, _>(|connection| {
            if !ledger_has_description(connection)? {
                connection.batch_execute("ALTER TABLE ledger_entries ADD COLUMN description TEXT CHECK (description IS NULL OR length(description) <= 500)")?;
            }
            Ok(())
        })?;
    }
    if !todo_has_rollover(&mut connection)? {
        connection.immediate_transaction::<_, diesel::result::Error, _>(|connection| {
            if !todo_has_rollover(connection)? {
                connection.batch_execute("ALTER TABLE todo_items ADD COLUMN rollover BOOLEAN NOT NULL DEFAULT 0 CHECK (rollover IN (0, 1))")?;
            }
            Ok(())
        })?;
    }
    Ok(connection)
}

fn todo_has_rollover(connection: &mut SqliteConnection) -> QueryResult<bool> {
    #[derive(QueryableByName)]
    struct Column {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let columns = diesel::sql_query("PRAGMA table_info(todo_items)").load::<Column>(connection)?;
    Ok(columns.iter().any(|column| column.name == "rollover"))
}

fn ledger_has_description(connection: &mut SqliteConnection) -> QueryResult<bool> {
    #[derive(QueryableByName)]
    struct Column {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let columns =
        diesel::sql_query("PRAGMA table_info(ledger_entries)").load::<Column>(connection)?;
    Ok(columns.iter().any(|column| column.name == "description"))
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

#[cfg(test)]
mod rollover_migration_tests {
    use super::*;

    #[test]
    fn migrates_existing_tasks_without_enabling_rollover() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        let mut connection = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        let old_schema = include_str!("schema.sql").replace(
            "    rollover BOOLEAN NOT NULL DEFAULT 0 CHECK (rollover IN (0, 1)),\n",
            "",
        );
        connection.batch_execute(&old_schema).unwrap();
        connection.batch_execute("INSERT INTO todo_items (date,id,position,text,completed) VALUES ('2026-09-12','old',0,'Existing',0)").unwrap();
        drop(connection);
        #[derive(QueryableByName)]
        struct Record {
            #[diesel(sql_type = diesel::sql_types::Text)]
            text: String,
            #[diesel(sql_type = diesel::sql_types::Bool)]
            rollover: bool,
        }
        for _ in 0..2 {
            let mut connection = open(&path).unwrap();
            let record = diesel::sql_query("SELECT text, rollover FROM todo_items WHERE id='old'")
                .get_result::<Record>(&mut connection)
                .unwrap();
            assert_eq!(record.text, "Existing");
            assert!(!record.rollover);
        }
    }
}
