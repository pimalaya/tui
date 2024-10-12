use dirs::home_dir;
use email::maildir::config::MaildirConfig;

use crate::{terminal::prompt, Result};

pub fn start(account_name: impl AsRef<str>) -> Result<MaildirConfig> {
    let account_name = account_name.as_ref();

    let default_root_dir = home_dir().map(|home| home.join("Mail").join(account_name));
    let root_dir = prompt::path("Maildir path:", default_root_dir)?;
    let maildirpp = prompt::bool("Enable Maildir++?", false)?;

    Ok(MaildirConfig {
        root_dir,
        maildirpp,
    })
}
