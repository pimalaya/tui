use email::sendmail::config::SendmailConfig;

use crate::{prompt, Result};

pub fn start() -> Result<SendmailConfig> {
    let cmd = prompt::text(
        "Sendmail-compatible shell command to send emails",
        Some("/usr/bin/msmtp"),
    )?;

    let config = SendmailConfig { cmd: cmd.into() };

    Ok(config)
}
