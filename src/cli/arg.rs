use std::path::PathBuf;

use shellexpand_utils::{canonicalize, expand};

/// Parse a string slice as [`PathBuf`]
///
/// The path is shell-expanded then canonicalized (if applicable).
pub fn path_parser(path: &str) -> Result<PathBuf, String> {
    expand::try_path(path)
        .map(canonicalize::path)
        .map_err(|err| err.to_string())
}
