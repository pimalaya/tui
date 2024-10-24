use email::sendmail::config::{SendmailConfig, SENDMAIL_DEFAULT_COMMAND};

use crate::{terminal::prompt, Result};

pub fn start() -> Result<SendmailConfig> {
    let cmd = prompt::text(
        "Sendmail-compatible shell command to send emails",
        Some(&SENDMAIL_DEFAULT_COMMAND),
    )?;

    let config = SendmailConfig {
        cmd: Some(cmd.into()),
    };

    Ok(config)
}
