use std::path::PathBuf;

use shellexpand_utils::{canonicalize, expand};

/// Parse a string slice as [`PathBuf`]
///
/// The path is shell-expanded then canonicalized (if applicable).
pub fn path_parser(path: &str) -> Result<PathBuf, String> {
    match expand::try_path(path) {
        Ok(path) => Ok(canonicalize::path(path)),
        Err(err) => Err(err.to_string()),
    }
}

#[macro_export]
macro_rules! long_version {
    () => {
        concat!(
            "v",
            env!("CARGO_PKG_VERSION"),
            " ",
            env!("CARGO_FEATURES"),
            "\nbuild: ",
            env!("CARGO_CFG_TARGET_OS"),
            " ",
            env!("CARGO_CFG_TARGET_ENV"),
            " ",
            env!("CARGO_CFG_TARGET_ARCH"),
            "\ngit: ",
            env!("GIT_DESCRIBE"),
            ", rev ",
            env!("GIT_REV"),
        )
    };
}
