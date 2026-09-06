#[cfg(debug_assertions)]
mod development;
#[cfg(all(not(debug_assertions), target_os = "macos"))]
mod macos;
#[cfg(not(debug_assertions))]
mod system;

#[cfg(debug_assertions)]
pub(crate) use development::{delete, read, save};
#[cfg(not(debug_assertions))]
pub(crate) use system::{delete, read, save};
