use crate::CredentialError;

pub fn load_development_environment() -> Result<(), CredentialError> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    if !path.exists() {
        return Ok(());
    }
    match dotenvy::from_path(&path) {
        Ok(()) => Ok(()),
        Err(source) => Err(CredentialError::DevelopmentFile { path, source }),
    }
}
