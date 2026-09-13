use crate::CredentialError;

pub fn load_dev_environment() -> Result<(), CredentialError> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    load(&path)
}

fn load(path: &std::path::Path) -> Result<(), CredentialError> {
    match dotenvy::from_path(path) {
        Ok(()) => Ok(()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        // dotenvy parse errors contain the original line, which may include a secret.
        Err(_) => Err(CredentialError::DevelopmentFile {
            path: path.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_environment_does_not_expose_values() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test.env");
        std::fs::write(&path, "MEMOS_API_KEY=\"synthetic-secret").unwrap();
        let error = super::load(&path).unwrap_err();
        assert!(error.to_string().contains("test.env"));
        assert!(!error.to_string().contains("synthetic-secret"));
        assert!(!format!("{error:?}").contains("synthetic-secret"));
    }
}
