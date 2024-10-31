use std::{path::Path, process::exit};

use crate::Result;

use super::{print, prompt};

#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;
#[cfg(feature = "sendmail")]
pub mod sendmail;
#[cfg(feature = "smtp")]
pub mod smtp;

pub fn confirm_or_exit(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    print::warn(format!("Cannot find configuration at {}.", path.display()));

    if !prompt::bool("Would you like to create one with the wizard?", true)? {
        exit(0);
    }

    Ok(())
}
