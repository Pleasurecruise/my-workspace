use std::error::Error;
use tracing_subscriber::EnvFilter;

pub use tracing::{debug, error, info, trace, warn};

pub fn init() -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()?;

    Ok(())
}
