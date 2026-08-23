use std::error::Error;
use tracing_subscriber::EnvFilter;

pub use tracing::{debug, error, info, trace, warn};

pub fn init() -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value)?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(error) => return Err(Box::new(error)),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()?;

    Ok(())
}
