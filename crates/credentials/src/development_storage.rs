use crate::{CredentialError, Stored};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Write;
use std::path::Path;

pub(crate) fn read<T: DeserializeOwned>(path: &Path) -> Result<Stored<T>, CredentialError> {
    let temporary = path.with_extension("json.tmp");
    if !path.exists() && temporary.exists() {
        replace(&temporary, path).map_err(|source| CredentialError::DevelopmentStorage {
            path: path.to_owned(),
            source,
        })?;
    }
    let encoded = match std::fs::read_to_string(path) {
        Ok(encoded) => encoded,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Stored::Missing),
        Err(source) => {
            return Err(CredentialError::DevelopmentStorage {
                path: path.to_owned(),
                source,
            });
        }
    };
    Ok(Stored::Ready(serde_json::from_str(&encoded)?))
}

pub(crate) fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), CredentialError> {
    let parent = path
        .parent()
        .ok_or(CredentialError::DevelopmentDataDirectory)?;
    std::fs::create_dir_all(parent).map_err(|source| CredentialError::DevelopmentStorage {
        path: parent.to_owned(),
        source,
    })?;
    let temporary = path.with_extension("json.tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file =
        options
            .open(&temporary)
            .map_err(|source| CredentialError::DevelopmentStorage {
                path: temporary.clone(),
                source,
            })?;
    let encoded = serde_json::to_vec(value)?;
    file.write_all(&encoded)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|source| CredentialError::DevelopmentStorage {
            path: temporary.clone(),
            source,
        })?;
    protect(&temporary)?;
    replace(&temporary, path).map_err(|source| CredentialError::DevelopmentStorage {
        path: path.to_owned(),
        source,
    })
}

#[cfg(unix)]
fn protect(path: &Path) -> Result<(), CredentialError> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|source| {
        CredentialError::DevelopmentStorage {
            path: path.to_owned(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn protect(_path: &Path) -> Result<(), CredentialError> {
    Ok(())
}

#[cfg(not(windows))]
fn replace(temporary: &Path, path: &Path) -> std::io::Result<()> {
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
    struct Fixture {
        value: String,
    }

    #[test]
    fn replaces_and_recovers_credentials() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("vesper-credentials-{unique}"));
        let path = directory.join("credentials.json");
        save(
            &path,
            &Fixture {
                value: "first".to_owned(),
            },
        )
        .unwrap();
        save(
            &path,
            &Fixture {
                value: "second".to_owned(),
            },
        )
        .unwrap();
        let temporary = path.with_extension("json.tmp");
        std::fs::rename(&path, &temporary).unwrap();

        let Stored::Ready(restored): Stored<Fixture> = read(&path).unwrap() else {
            panic!("saved credentials should be available");
        };
        assert_eq!(
            restored,
            Fixture {
                value: "second".to_owned()
            }
        );
        assert!(!temporary.exists());

        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(windows)]
fn replace(temporary: &Path, path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    std::fs::rename(temporary, path)
}
