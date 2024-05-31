use email::notmuch::config::NotmuchConfig;

use crate::{prompt, Result};

pub fn start() -> Result<NotmuchConfig> {
    let config = NotmuchConfig {
        database_path: Some(prompt::path("Notmuch database path:", None::<&str>)?),
        ..Default::default()
    };

    Ok(config)
}
