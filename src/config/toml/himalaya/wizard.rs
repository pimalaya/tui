use std::{fs, path::Path, process::exit};

use super::config::*;
use crate::{config::toml::TomlConfig, print, prompt, wizard, Error, Result};

const DEFAULT_BACKEND_KINDS: &[BackendKind] = &[
    #[cfg(feature = "imap")]
    BackendKind::Imap,
    #[cfg(feature = "maildir")]
    BackendKind::Maildir,
    #[cfg(feature = "notmuch")]
    BackendKind::Notmuch,
];

const SEND_MESSAGE_BACKEND_KINDS: &[BackendKind] = &[
    #[cfg(feature = "smtp")]
    BackendKind::Smtp,
    #[cfg(feature = "sendmail")]
    BackendKind::Sendmail,
];

pub fn confirm_or_exit(path: &Path) -> Result<()> {
    print::warn(format!("Cannot find configuration at {}.", path.display()));

    if !prompt::bool("Would you like to create one with the wizard?", true)? {
        exit(0);
    }

    Ok(())
}

pub async fn run(path: impl AsRef<Path>) -> Result<HimalayaTomlConfig> {
    print::section("Configuring your default account");

    let mut config = HimalayaTomlConfig::default();

    let email = prompt::email("Email address:", None)?;

    let mut account_config = HimalayaTomlAccountConfig {
        default: Some(true),
        email: email.to_string(),
        ..Default::default()
    };

    let autoconfig_email = account_config.email.to_owned();
    let autoconfig =
        tokio::spawn(async move { email::autoconfig::from_addr(&autoconfig_email).await.ok() });

    let default_account_name = email
        .domain()
        .split_once('.')
        .map(|domain| domain.0)
        .unwrap_or(email.domain());
    let account_name = prompt::text("Account name:", Some(default_account_name))?;

    account_config.display_name = Some(prompt::text(
        "Full display name:",
        Some(email.local_part()),
    )?);

    account_config.downloads_dir = Some(prompt::path("Downloads directory:", Some("~/Downloads"))?);

    let autoconfig = autoconfig.await?;
    let autoconfig = autoconfig.as_ref();

    if let Some(config) = autoconfig {
        if config.is_gmail() {
            println!();
            print::warn("Warning: Google passwords cannot be used directly, see:");
            print::warn("https://github.com/pimalaya/himalaya?tab=readme-ov-file#configuration");
            println!();
        }
    }

    let backend = prompt::item("Default backend:", &*DEFAULT_BACKEND_KINDS, None)?;

    match backend {
        #[cfg(feature = "imap")]
        BackendKind::Imap => {
            let imap_config = wizard::imap::start(&account_name, &email, autoconfig).await?;
            account_config.imap = Some(imap_config);
            account_config.backend = Some(BackendKind::Imap);
        }
        #[cfg(feature = "maildir")]
        BackendKind::Maildir => {
            let mdir_config = wizard::maildir::start(&account_name)?;
            account_config.maildir = Some(mdir_config);
            account_config.backend = Some(BackendKind::Maildir);
        }
        #[cfg(feature = "notmuch")]
        BackendKind::Notmuch => {
            let notmuch_config = wizard::notmuch::start()?;
            account_config.notmuch = Some(notmuch_config);
            account_config.backend = Some(BackendKind::Notmuch);
        }
        _ => (),
    }

    let backend = prompt::item(
        "Backend for sending messages:",
        &*SEND_MESSAGE_BACKEND_KINDS,
        None,
    )?;

    match backend {
        #[cfg(feature = "smtp")]
        BackendKind::Smtp => {
            let smtp_config = wizard::smtp::start(&account_name, &email, autoconfig).await?;
            account_config.smtp = Some(smtp_config);
            account_config.message = Some(MessageConfig {
                send: Some(SendMessageConfig {
                    backend: Some(BackendKind::Smtp),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        #[cfg(feature = "sendmail")]
        BackendKind::Sendmail => {
            let sendmail_config = wizard::sendmail::start()?;
            account_config.sendmail = Some(sendmail_config);
            account_config.message = Some(MessageConfig {
                send: Some(SendMessageConfig {
                    backend: Some(BackendKind::Sendmail),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        _ => (),
    };

    config.accounts.insert(account_name, account_config);

    let path = prompt::path("Where to save the configuration?", Some(path))?;
    println!("Writing configuration to {}…", path.display());

    let toml = config.pretty_serialize()?;
    fs::create_dir_all(path.parent().unwrap_or(&path))
        .map_err(|err| Error::CreateConfigFileError(err, path.clone()))?;
    fs::write(&path, toml).map_err(|err| Error::WriteConfigFileError(err, path.clone()))?;

    println!("Done! Exiting the wizard…");
    Ok(config)
}
