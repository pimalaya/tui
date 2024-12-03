use std::{
    fmt,
    path::{Path, PathBuf},
};

use super::config::*;
use crate::{
    terminal::{config::TomlConfig, print, prompt, wizard},
    Result,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendKind {
    None,
    #[cfg(feature = "imap")]
    Imap,
    #[cfg(feature = "maildir")]
    Maildir,
    #[cfg(feature = "notmuch")]
    Notmuch,
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "None",
                #[cfg(feature = "imap")]
                Self::Imap => "IMAP",
                #[cfg(feature = "maildir")]
                Self::Maildir => "Maildir",
                #[cfg(feature = "notmuch")]
                Self::Notmuch => "Notmuch",
            }
        )
    }
}

const DEFAULT_BACKEND_KINDS: &[BackendKind] = &[
    BackendKind::None,
    #[cfg(feature = "imap")]
    BackendKind::Imap,
    #[cfg(feature = "maildir")]
    BackendKind::Maildir,
    #[cfg(feature = "notmuch")]
    BackendKind::Notmuch,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SendingBackendKind {
    None,
    #[cfg(feature = "smtp")]
    Smtp,
    #[cfg(feature = "sendmail")]
    Sendmail,
}

impl fmt::Display for SendingBackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "None",
                #[cfg(feature = "smtp")]
                Self::Smtp => "SMTP",
                #[cfg(feature = "sendmail")]
                Self::Sendmail => "Sendmail",
            }
        )
    }
}

const SEND_MESSAGE_BACKEND_KINDS: &[SendingBackendKind] = &[
    SendingBackendKind::None,
    #[cfg(feature = "smtp")]
    SendingBackendKind::Smtp,
    #[cfg(feature = "sendmail")]
    SendingBackendKind::Sendmail,
];

pub async fn edit(
    path: impl AsRef<Path>,
    mut config: HimalayaTomlConfig,
    account_name: Option<&str>,
    mut account_config: HimalayaTomlAccountConfig,
) -> Result<HimalayaTomlConfig> {
    match account_name.as_ref() {
        Some(name) => print::section(format!("Configuring your account {name}")),
        None => print::section("Configuring your default account"),
    };

    let default_email = Some(account_config.email.as_str()).filter(|email| !email.is_empty());
    let email = prompt::email("Email address:", default_email)?;

    account_config.email = email.to_string();

    let default =
        account_name.is_none() || prompt::bool("Should this account be the default one?", false)?;

    if default {
        config
            .accounts
            .iter_mut()
            .for_each(|(_, config)| config.default = None)
    }

    account_config.default = Some(default);

    let autoconfig_email = account_config.email.to_owned();
    let autoconfig =
        tokio::spawn(async move { email::autoconfig::from_addr(&autoconfig_email).await.ok() });

    let default_account_name = match account_name {
        Some(name) => name,
        None => email
            .domain()
            .split_once('.')
            .map(|domain| domain.0)
            .unwrap_or(email.domain()),
    };

    let account_name = prompt::text("Account name:", Some(default_account_name))?;

    let default_display_name = account_config
        .display_name
        .as_deref()
        .or(Some(email.local_part()));

    account_config.display_name = Some(prompt::text("Full display name:", default_display_name)?);

    let default_downloads_dir = Some(PathBuf::from("~/Downloads"));
    let default_downloads_dir = account_config
        .downloads_dir
        .as_deref()
        .or(default_downloads_dir.as_deref());

    account_config.downloads_dir =
        Some(prompt::path("Downloads directory:", default_downloads_dir)?);

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
        BackendKind::None => {
            account_config.backend = Some(Backend::None);
        }
        #[cfg(feature = "imap")]
        BackendKind::Imap => {
            let config = wizard::imap::start(&account_name, &email, autoconfig).await?;
            account_config.backend = Some(Backend::Imap(config));
        }
        #[cfg(feature = "maildir")]
        BackendKind::Maildir => {
            let config = wizard::maildir::start(&account_name)?;
            account_config.backend = Some(Backend::Maildir(config));
        }
        #[cfg(feature = "notmuch")]
        BackendKind::Notmuch => {
            let config = wizard::notmuch::start()?;
            account_config.backend = Some(Backend::Notmuch(config));
        }
    }

    let backend = prompt::item(
        "Backend for sending messages:",
        &*SEND_MESSAGE_BACKEND_KINDS,
        None,
    )?;

    match backend {
        SendingBackendKind::None => {
            account_config.message = Some(MessageConfig {
                send: Some(SendMessageConfig {
                    backend: Some(SendingBackend::None),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        #[cfg(feature = "smtp")]
        SendingBackendKind::Smtp => {
            let config = wizard::smtp::start(&account_name, &email, autoconfig).await?;
            account_config.message = Some(MessageConfig {
                send: Some(SendMessageConfig {
                    backend: Some(SendingBackend::Smtp(config)),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        #[cfg(feature = "sendmail")]
        SendingBackendKind::Sendmail => {
            let config = wizard::sendmail::start()?;
            account_config.message = Some(MessageConfig {
                send: Some(SendMessageConfig {
                    backend: Some(SendingBackend::Sendmail(config)),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
    };

    config.accounts.insert(account_name, account_config);
    config.write(path.as_ref())?;

    Ok(config)
}
