use crate::Error;

pub fn load_dev_environment() -> Result<(), Error> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    load(&path)
}

/// Reads a development override that must be set completely or not at all.
pub fn variables<const N: usize>(
    names: [&str; N],
    label: &'static str,
) -> Result<Option<[String; N]>, Error> {
    let values = names.map(std::env::var_os);
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    let mut strings: [String; N] = std::array::from_fn(|_| String::new());
    for (slot, value) in strings.iter_mut().zip(values) {
        let Some(value) = value else {
            return Err(Error::IncompleteDevelopment(label));
        };
        *slot = value
            .into_string()
            .map_err(|_| Error::InvalidDevelopment(label))?;
    }
    Ok(Some(strings))
}

fn load(path: &std::path::Path) -> Result<(), Error> {
    match dotenvy::from_path(path) {
        Ok(()) => Ok(()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        // dotenvy parse errors contain the original line, which may include a secret.
        Err(_) => Err(Error::DevelopmentFile {
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

    #[test]
    fn partial_overrides_are_incomplete() {
        let result = super::variables(["VESPER_TEST_PARTIAL_A", "PATH"], "test");
        assert!(matches!(
            result,
            Err(crate::Error::IncompleteDevelopment("test"))
        ));
        assert!(matches!(
            super::variables(["VESPER_TEST_MISSING_A", "VESPER_TEST_MISSING_B"], "test"),
            Ok(None)
        ));
    }
}
