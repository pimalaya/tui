use std::{io, result};

use inquire::InquireError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "wizard")]
    #[error("cannot create TOML config parent directory at {1}")]
    CreateTomlConfigParentDirectoryError(#[source] std::io::Error, std::path::PathBuf),
    #[cfg(feature = "wizard")]
    #[error("cannot write TOML config at {1}")]
    WriteTomlConfigError(#[source] std::io::Error, std::path::PathBuf),

    #[cfg(feature = "config")]
    #[error("cannot create TOML config from invalid or missing paths")]
    CreateTomlConfigFromInvalidPathsError,
    #[cfg(feature = "config")]
    #[error("cannot create TOML config from wizard")]
    CreateTomlConfigFromWizardError(#[source] color_eyre::eyre::Error),
    #[error("cannot prompt unsigned integer (u16)")]
    PromptU16Error(#[source] InquireError),
    #[error("cannot prompt unsigned integer (usize)")]
    PromptUsizeError(#[source] InquireError),
    #[error("cannot prompt secret")]
    PromptSecretError(#[source] InquireError),
    #[error("cannot prompt password")]
    PromptPasswordError(#[source] InquireError),
    #[error("cannot prompt text")]
    PromptTextError(#[source] InquireError),
    #[error("cannot prompt boolean")]
    PromptBoolError(#[source] InquireError),
    #[error("cannot prompt item from list")]
    PromptItemError(#[source] InquireError),
    #[cfg(feature = "email")]
    #[error("cannot prompt email")]
    PromptEmailError(#[source] InquireError),
    #[cfg(feature = "path")]
    #[error("cannot prompt path")]
    PromptPathError(#[source] InquireError),

    #[cfg(feature = "oauth2")]
    #[error(transparent)]
    OAuth2Error(#[from] oauth::v2_0::Error),
    #[cfg(feature = "imap")]
    #[error(transparent)]
    AccountError(#[from] email::account::Error),
    #[cfg(feature = "imap")]
    #[error(transparent)]
    ImapError(#[from] email::imap::Error),
    #[cfg(feature = "smtp")]
    #[error(transparent)]
    SmtpError(#[from] email::smtp::Error),
    #[cfg(feature = "imap")]
    #[error(transparent)]
    SecretError(#[from] secret::Error),

    #[cfg(feature = "wizard")]
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[cfg(feature = "config")]
    #[error("cannot read config file from empty paths")]
    ReadTomlConfigFileFromEmptyPaths,
    #[cfg(feature = "config")]
    #[error("cannot read config file at {}", .1.display())]
    ReadTomlConfigFile(#[source] std::io::Error, std::path::PathBuf),
    #[cfg(feature = "config")]
    #[error("cannot parse config file at {}", .1.display())]
    ParseTomlConfigFile(#[source] toml::de::Error, std::path::PathBuf),
    #[cfg(feature = "config")]
    #[error("cannot merge config files: {0}")]
    MergeTomlConfigFiles(serde_toml_merge::Error),
    #[cfg(feature = "config")]
    #[error("cannot get XDG config directory")]
    GetXdgConfigDirectory,
    #[cfg(feature = "config")]
    #[error("cannot serialize TOML config")]
    SerializeTomlConfigError(#[source] toml::ser::Error),
    #[cfg(feature = "config")]
    #[error("cannot parse serialized TOML config as document")]
    ParseSerializedTomlConfigError(#[source] toml_edit::TomlError),
    #[cfg(feature = "config")]
    #[error("cannot find default account configuration")]
    GetDefaultAccountConfigError,
    #[cfg(feature = "config")]
    #[error("cannot find configuration for account {0}")]
    GetAccountConfigError(String),
    #[cfg(all(feature = "config", feature = "himalaya"))]
    #[error("cannot create config file {}", .1.display())]
    CreateConfigFileError(#[source] std::io::Error, std::path::PathBuf),
    #[cfg(all(feature = "config", feature = "himalaya"))]
    #[error("cannot write config to file {}", .1.display())]
    WriteConfigFileError(#[source] std::io::Error, std::path::PathBuf),
}

pub type Result<T> = result::Result<T, Error>;

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::InvalidInput, err)
    }
}
