mod error;
#[cfg(feature = "himalaya")]
pub mod himalaya;
pub mod terminal;

pub use error::{Error, Result};
