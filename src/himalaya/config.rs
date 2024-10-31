use std::{
    collections::{hash_map::Iter, HashMap, HashSet},
    fmt,
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use color_eyre::Result;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Row, Table};
use crossterm::{
    cursor,
    style::{Color, Stylize},
    terminal,
};
#[cfg(feature = "pgp")]
use email::account::config::pgp::PgpConfig;
#[cfg(feature = "imap")]
use email::imap::config::{ImapAuthConfig, ImapConfig};
#[cfg(feature = "maildir")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "notmuch")]
use email::notmuch::config::NotmuchConfig;
#[cfg(feature = "sendmail")]
use email::sendmail::config::SendmailConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::{SmtpAuthConfig, SmtpConfig};
use email::{
    account::config::AccountConfig,
    config::Config,
    envelope::ThreadedEnvelope,
    message::{
        add::config::MessageWriteConfig, delete::config::DeleteMessageConfig,
        get::config::MessageReadConfig,
    },
    template::config::TemplateConfig,
};
use petgraph::graphmap::DiGraphMap;
use process::Command;
use serde::{Deserialize, Serialize, Serializer};

use super::id_mapper::IdMapper;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct HimalayaTomlConfig {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub accounts: HashMap<String, HimalayaTomlAccountConfig>,
    pub account: Option<AccountsConfig>,
}

impl From<HimalayaTomlConfig> for Config {
    fn from(config: HimalayaTomlConfig) -> Self {
        Self {
            display_name: config.display_name,
            signature: config.signature,
            signature_delim: config.signature_delim,
            downloads_dir: config.downloads_dir,
            accounts: config
                .accounts
                .into_iter()
                .map(|(name, config)| {
                    let mut config = AccountConfig::from(config);
                    config.name = name.clone();
                    (name, config)
                })
                .collect(),
        }
    }
}

impl HimalayaTomlConfig {
    pub fn account_list_table_preset(&self) -> Option<String> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.preset.clone())
    }

    pub fn account_list_table_name_color(&self) -> Option<Color> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.name_color)
    }

    pub fn account_list_table_backends_color(&self) -> Option<Color> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.backends_color)
    }

    pub fn account_list_table_default_color(&self) -> Option<Color> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.default_color)
    }
}

#[async_trait]
impl crate::terminal::config::TomlConfig for HimalayaTomlConfig {
    type TomlAccountConfig = HimalayaTomlAccountConfig;

    fn project_name() -> &'static str {
        "himalaya"
    }

    fn get_default_account_config(&self) -> Option<(String, Self::TomlAccountConfig)> {
        self.accounts.iter().find_map(|(name, account)| {
            account
                .default
                .filter(|default| *default)
                .map(|_| (name.to_owned(), account.clone()))
        })
    }

    fn get_account_config(&self, name: &str) -> Option<(String, Self::TomlAccountConfig)> {
        self.accounts
            .get(name)
            .map(|account| (name.to_owned(), account.clone()))
    }

    #[cfg(feature = "wizard")]
    async fn from_wizard(path: &std::path::Path) -> color_eyre::Result<Self> {
        crate::terminal::wizard::confirm_or_exit(path)?;
        Ok(super::wizard::run(path).await?)
    }

    fn to_toml_account_config(
        &self,
        account_name: Option<&str>,
    ) -> crate::Result<(String, Self::TomlAccountConfig)> {
        #[allow(unused_mut)]
        let (name, mut config) = match account_name {
            Some("default") | Some("") | None => self
                .get_default_account_config()
                .ok_or(crate::Error::GetDefaultAccountConfigError),
            Some(name) => self
                .get_account_config(name)
                .ok_or_else(|| crate::Error::GetAccountConfigError(name.to_owned())),
        }?;

        #[cfg(all(feature = "imap", feature = "keyring"))]
        if let Some(Backend::Imap(imap_config)) = config.backend.as_mut() {
            imap_config.auth.replace_empty_secrets(&name)?;
        }

        #[cfg(all(feature = "smtp", feature = "keyring"))]
        if let Some(SendingBackend::Smtp(smtp_config)) = config.message_send_backend_mut() {
            smtp_config.auth.replace_empty_secrets(&name)?;
        }

        Ok((name, config))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HimalayaTomlAccountConfig {
    pub default: Option<bool>,
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub backend: Option<Backend>,

    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,
    #[cfg(not(feature = "pgp"))]
    #[serde(default)]
    #[serde(skip_serializing, deserialize_with = "missing_pgp_feature")]
    pub pgp: Option<()>,

    pub folder: Option<FolderConfig>,
    pub envelope: Option<EnvelopeConfig>,
    pub message: Option<MessageConfig>,
    pub template: Option<TemplateConfig>,
}

#[cfg(not(feature = "pgp"))]
fn missing_pgp_feature<'de, D: serde::Deserializer<'de>>(_: D) -> Result<Option<()>, D::Error> {
    Err(serde::de::Error::custom(
        "missing `pgp-commands`, `pgp-gpg` or `pgp-native` cargo feature",
    ))
}

impl From<HimalayaTomlAccountConfig> for AccountConfig {
    fn from(config: HimalayaTomlAccountConfig) -> Self {
        Self {
            name: String::new(),
            email: config.email,
            display_name: config.display_name,
            signature: config.signature,
            signature_delim: config.signature_delim,
            downloads_dir: config.downloads_dir,

            #[cfg(feature = "pgp")]
            pgp: config.pgp,

            folder: config.folder.map(Into::into),
            envelope: config.envelope.map(Into::into),
            flag: None,
            message: config.message.map(Into::into),
            template: config.template,
        }
    }
}

impl HimalayaTomlAccountConfig {
    pub fn folder_list_table_preset(&self) -> Option<String> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.preset.clone())
    }

    pub fn folder_list_table_name_color(&self) -> Option<Color> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.name_color)
    }

    pub fn folder_list_table_desc_color(&self) -> Option<Color> {
        self.folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.desc_color)
    }

    pub fn envelope_list_table_preset(&self) -> Option<String> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.preset.clone())
    }

    pub fn envelope_list_table_unseen_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.unseen_char)
    }

    pub fn envelope_list_table_replied_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.replied_char)
    }

    pub fn envelope_list_table_flagged_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.flagged_char)
    }

    pub fn envelope_list_table_attachment_char(&self) -> Option<char> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.attachment_char)
    }

    pub fn envelope_list_table_id_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.id_color)
    }

    pub fn envelope_list_table_flags_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.flags_color)
    }

    pub fn envelope_list_table_subject_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.subject_color)
    }

    pub fn envelope_list_table_sender_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.sender_color)
    }

    pub fn envelope_list_table_date_color(&self) -> Option<Color> {
        self.envelope
            .as_ref()
            .and_then(|env| env.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.date_color)
    }

    pub fn message_send_backend(&self) -> Option<&SendingBackend> {
        self.message
            .as_ref()
            .and_then(|msg| msg.send.as_ref())
            .and_then(|send| send.backend.as_ref())
    }

    pub fn message_send_backend_mut(&mut self) -> Option<&mut SendingBackend> {
        self.message
            .as_mut()
            .and_then(|msg| msg.send.as_mut())
            .and_then(|send| send.backend.as_mut())
    }

    #[cfg(feature = "imap")]
    pub fn imap_config(&self) -> Option<&ImapConfig> {
        self.backend.as_ref().and_then(|backend| match backend {
            Backend::Imap(config) => Some(config),
            _ => None,
        })
    }

    #[cfg(feature = "imap")]
    pub fn imap_auth_config(&self) -> Option<&ImapAuthConfig> {
        self.imap_config().map(|imap| &imap.auth)
    }

    #[cfg(feature = "smtp")]
    pub fn smtp_config(&self) -> Option<&SmtpConfig> {
        self.message_send_backend()
            .and_then(|backend| match backend {
                SendingBackend::Smtp(config) => Some(config),
                _ => None,
            })
    }

    #[cfg(feature = "smtp")]
    pub fn smtp_auth_config(&self) -> Option<&SmtpAuthConfig> {
        self.smtp_config().map(|smtp| &smtp.auth)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountsConfig {
    pub list: Option<ListAccountsConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListAccountsConfig {
    pub table: Option<ListAccountsTableConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListAccountsTableConfig {
    pub preset: Option<String>,
    pub name_color: Option<Color>,
    pub backends_color: Option<Color>,
    pub default_color: Option<Color>,
}

impl ListAccountsTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn name_color(&self) -> comfy_table::Color {
        map_color(self.name_color.unwrap_or(Color::Green))
    }

    pub fn backends_color(&self) -> comfy_table::Color {
        map_color(self.backends_color.unwrap_or(Color::Blue))
    }

    pub fn default_color(&self) -> comfy_table::Color {
        map_color(self.default_color.unwrap_or(Color::Reset))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", from = "BackendDerive")]
pub enum Backend {
    #[default]
    #[serde(skip)]
    None,
    #[cfg(feature = "imap")]
    Imap(ImapConfig),
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(feature = "notmuch")]
    Notmuch(NotmuchConfig),
}

impl ToString for Backend {
    fn to_string(&self) -> String {
        match self {
            Self::None => String::from("None"),
            #[cfg(feature = "imap")]
            Self::Imap(_) => String::from("IMAP"),
            #[cfg(feature = "maildir")]
            Self::Maildir(_) => String::from("Maildir"),
            #[cfg(feature = "notmuch")]
            Self::Notmuch(_) => String::from("Notmuch"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum BackendDerive {
    #[cfg(feature = "imap")]
    Imap(ImapConfig),
    #[cfg(not(feature = "imap"))]
    #[serde(skip_serializing, deserialize_with = "missing_imap_feature")]
    Imap,

    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(not(feature = "maildir"))]
    #[serde(skip_serializing, deserialize_with = "missing_maildir_feature")]
    Maildir,

    #[cfg(feature = "notmuch")]
    Notmuch(NotmuchConfig),
    #[cfg(not(feature = "notmuch"))]
    #[serde(skip_serializing, deserialize_with = "missing_notmuch_feature")]
    Notmuch,
}

impl From<BackendDerive> for Backend {
    fn from(backend: BackendDerive) -> Backend {
        match backend {
            #[cfg(feature = "imap")]
            BackendDerive::Imap(config) => Backend::Imap(config),
            #[cfg(not(feature = "imap"))]
            BackendDerive::Imap => Backend::None,

            #[cfg(feature = "maildir")]
            BackendDerive::Maildir(config) => Backend::Maildir(config),
            #[cfg(not(feature = "maildir"))]
            BackendDerive::Maildir => Backend::None,

            #[cfg(feature = "notmuch")]
            BackendDerive::Notmuch(config) => Backend::Notmuch(config),
            #[cfg(not(feature = "notmuch"))]
            BackendDerive::Notmuch => Backend::None,
        }
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", from = "SendingBackendDerive")]
pub enum SendingBackend {
    #[default]
    #[serde(skip)]
    None,
    #[cfg(feature = "smtp")]
    Smtp(SmtpConfig),
    #[cfg(feature = "sendmail")]
    Sendmail(SendmailConfig),
}

impl ToString for SendingBackend {
    fn to_string(&self) -> String {
        match self {
            Self::None => String::from("None"),
            #[cfg(feature = "smtp")]
            Self::Smtp(_) => String::from("SMTP"),
            #[cfg(feature = "sendmail")]
            Self::Sendmail(_) => String::from("Sendmail"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum SendingBackendDerive {
    #[cfg(feature = "smtp")]
    Smtp(SmtpConfig),
    #[cfg(not(feature = "smtp"))]
    #[serde(skip_serializing, deserialize_with = "missing_smtp_feature")]
    Smtp,

    #[cfg(feature = "sendmail")]
    Sendmail(SendmailConfig),
    #[cfg(not(feature = "sendmail"))]
    #[serde(skip_serializing, deserialize_with = "missing_sendmail_feature")]
    Sendmail,
}

impl From<SendingBackendDerive> for SendingBackend {
    fn from(backend: SendingBackendDerive) -> SendingBackend {
        match backend {
            #[cfg(feature = "smtp")]
            SendingBackendDerive::Smtp(config) => SendingBackend::Smtp(config),
            #[cfg(not(feature = "smtp"))]
            SendingBackendDerive::Smtp => SendingBackend::None,

            #[cfg(feature = "sendmail")]
            SendingBackendDerive::Sendmail(config) => SendingBackend::Sendmail(config),
            #[cfg(not(feature = "sendmail"))]
            SendingBackendDerive::Sendmail => SendingBackend::None,
        }
    }
}

#[cfg(not(feature = "imap"))]
fn missing_imap_feature<'de, D: serde::Deserializer<'de>, T>(_: D) -> Result<T, D::Error> {
    Err(serde::de::Error::custom("missing `imap` cargo feature"))
}

#[cfg(not(feature = "maildir"))]
fn missing_maildir_feature<'de, D: serde::Deserializer<'de>, T>(_: D) -> Result<T, D::Error> {
    Err(serde::de::Error::custom("missing `maildir` cargo feature"))
}

#[cfg(not(feature = "notmuch"))]
fn missing_notmuch_feature<'de, D: serde::Deserializer<'de>, T>(_: D) -> Result<T, D::Error> {
    Err(serde::de::Error::custom("missing `notmuch` cargo feature"))
}

#[cfg(not(feature = "smtp"))]
fn missing_smtp_feature<'de, D: serde::Deserializer<'de>, T>(_: D) -> Result<T, D::Error> {
    Err(serde::de::Error::custom("missing `smtp` cargo feature"))
}

#[cfg(not(feature = "sendmail"))]
fn missing_sendmail_feature<'de, D: serde::Deserializer<'de>, T>(_: D) -> Result<T, D::Error> {
    Err(serde::de::Error::custom("missing `sendmail` cargo feature"))
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvelopeConfig {
    pub list: Option<ListEnvelopesConfig>,
}

impl From<EnvelopeConfig> for email::envelope::config::EnvelopeConfig {
    fn from(config: EnvelopeConfig) -> Self {
        Self {
            list: config.list.map(Into::into),
            ..Default::default()
        }
    }
}

impl EnvelopeConfig {
    pub fn list_table_preset(&self) -> Option<String> {
        self.list
            .as_ref()
            .and_then(|c| c.table.as_ref())
            .and_then(|c| c.preset.clone())
    }

    pub fn list_table_unseen_char(&self) -> Option<char> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.unseen_char)
    }

    pub fn list_table_replied_char(&self) -> Option<char> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.replied_char)
    }

    pub fn list_table_flagged_char(&self) -> Option<char> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.flagged_char)
    }

    pub fn list_table_attachment_char(&self) -> Option<char> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.attachment_char)
    }

    pub fn list_table_id_color(&self) -> Option<Color> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.id_color)
    }

    pub fn list_table_flags_color(&self) -> Option<Color> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.flags_color)
    }

    pub fn list_table_subject_color(&self) -> Option<Color> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.subject_color)
    }

    pub fn list_table_sender_color(&self) -> Option<Color> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.sender_color)
    }

    pub fn list_table_date_color(&self) -> Option<Color> {
        self.list
            .as_ref()
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.date_color)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListEnvelopesConfig {
    pub page_size: Option<usize>,
    pub datetime_fmt: Option<String>,
    pub datetime_local_tz: Option<bool>,
    pub table: Option<ListEnvelopesTableConfig>,
}

impl From<ListEnvelopesConfig> for email::envelope::list::config::EnvelopeListConfig {
    fn from(config: ListEnvelopesConfig) -> Self {
        Self {
            page_size: config.page_size,
            datetime_fmt: config.datetime_fmt,
            datetime_local_tz: config.datetime_local_tz,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListEnvelopesTableConfig {
    pub preset: Option<String>,

    pub unseen_char: Option<char>,
    pub replied_char: Option<char>,
    pub flagged_char: Option<char>,
    pub attachment_char: Option<char>,

    pub id_color: Option<Color>,
    pub flags_color: Option<Color>,
    pub subject_color: Option<Color>,
    pub sender_color: Option<Color>,
    pub date_color: Option<Color>,
}

impl ListEnvelopesTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn replied_char(&self, replied: bool) -> char {
        if replied {
            self.replied_char.unwrap_or('R')
        } else {
            ' '
        }
    }

    pub fn flagged_char(&self, flagged: bool) -> char {
        if flagged {
            self.flagged_char.unwrap_or('!')
        } else {
            ' '
        }
    }

    pub fn attachment_char(&self, attachment: bool) -> char {
        if attachment {
            self.attachment_char.unwrap_or('@')
        } else {
            ' '
        }
    }

    pub fn unseen_char(&self, unseen: bool) -> char {
        if unseen {
            self.unseen_char.unwrap_or('*')
        } else {
            ' '
        }
    }

    pub fn id_color(&self) -> comfy_table::Color {
        map_color(self.id_color.unwrap_or(Color::Red))
    }

    pub fn flags_color(&self) -> comfy_table::Color {
        map_color(self.flags_color.unwrap_or(Color::Reset))
    }

    pub fn subject_color(&self) -> comfy_table::Color {
        map_color(self.subject_color.unwrap_or(Color::Green))
    }

    pub fn sender_color(&self) -> comfy_table::Color {
        map_color(self.sender_color.unwrap_or(Color::Blue))
    }

    pub fn date_color(&self) -> comfy_table::Color {
        map_color(self.date_color.unwrap_or(Color::DarkYellow))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FolderConfig {
    pub aliases: Option<HashMap<String, String>>,
    pub list: Option<ListFoldersConfig>,
}

impl From<FolderConfig> for email::folder::config::FolderConfig {
    fn from(config: FolderConfig) -> Self {
        Self {
            aliases: config.aliases,
            list: config.list.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListFoldersConfig {
    pub table: Option<ListFoldersTableConfig>,
    pub page_size: Option<usize>,
}

impl From<ListFoldersConfig> for email::folder::list::config::FolderListConfig {
    fn from(config: ListFoldersConfig) -> Self {
        Self {
            page_size: config.page_size,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListFoldersTableConfig {
    pub preset: Option<String>,
    pub name_color: Option<Color>,
    pub desc_color: Option<Color>,
}

impl ListFoldersTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn name_color(&self) -> comfy_table::Color {
        map_color(self.name_color.unwrap_or(Color::Blue))
    }

    pub fn desc_color(&self) -> comfy_table::Color {
        map_color(self.desc_color.unwrap_or(Color::Green))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageConfig {
    pub read: Option<MessageReadConfig>,
    pub write: Option<MessageWriteConfig>,
    pub send: Option<SendMessageConfig>,
    pub delete: Option<DeleteMessageConfig>,
}

impl From<MessageConfig> for email::message::config::MessageConfig {
    fn from(config: MessageConfig) -> Self {
        Self {
            read: config.read,
            write: config.write,
            send: config.send.map(Into::into),
            delete: config.delete,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SendMessageConfig {
    pub backend: Option<SendingBackend>,
    pub save_copy: Option<bool>,
    pub pre_hook: Option<Command>,
}

impl From<SendMessageConfig> for email::message::send::config::MessageSendConfig {
    fn from(config: SendMessageConfig) -> Self {
        Self {
            save_copy: config.save_copy,
            pre_hook: config.pre_hook,
        }
    }
}

fn map_color(color: Color) -> comfy_table::Color {
    match color {
        Color::Reset => comfy_table::Color::Reset,
        Color::Black => comfy_table::Color::Black,
        Color::DarkGrey => comfy_table::Color::DarkGrey,
        Color::Red => comfy_table::Color::Red,
        Color::DarkRed => comfy_table::Color::DarkRed,
        Color::Green => comfy_table::Color::Green,
        Color::DarkGreen => comfy_table::Color::DarkGreen,
        Color::Yellow => comfy_table::Color::Yellow,
        Color::DarkYellow => comfy_table::Color::DarkYellow,
        Color::Blue => comfy_table::Color::Blue,
        Color::DarkBlue => comfy_table::Color::DarkBlue,
        Color::Magenta => comfy_table::Color::Magenta,
        Color::DarkMagenta => comfy_table::Color::DarkMagenta,
        Color::Cyan => comfy_table::Color::Cyan,
        Color::DarkCyan => comfy_table::Color::DarkCyan,
        Color::White => comfy_table::Color::White,
        Color::Grey => comfy_table::Color::Grey,
        Color::Rgb { r, g, b } => comfy_table::Color::Rgb { r, g, b },
        Color::AnsiValue(n) => comfy_table::Color::AnsiValue(n),
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folder {
    pub name: String,
    pub desc: String,
}

impl Folder {
    pub fn to_row(&self, config: &ListFoldersTableConfig) -> Row {
        let mut row = Row::new();
        row.max_height(1);

        row.add_cell(Cell::new(&self.name).fg(config.name_color()));
        row.add_cell(Cell::new(&self.desc).fg(config.desc_color()));

        row
    }
}

impl From<email::folder::Folder> for Folder {
    fn from(folder: email::folder::Folder) -> Self {
        Folder {
            name: folder.name,
            desc: folder.desc,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Folders(Vec<Folder>);

impl Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::folder::Folders> for Folders {
    fn from(folders: email::folder::Folders) -> Self {
        Folders(folders.into_iter().map(Folder::from).collect())
    }
}

pub struct FoldersTable {
    folders: Folders,
    width: Option<u16>,
    config: ListFoldersTableConfig,
}

impl FoldersTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_name_color(mut self, color: Option<Color>) -> Self {
        self.config.name_color = color;
        self
    }

    pub fn with_some_desc_color(mut self, color: Option<Color>) -> Self {
        self.config.desc_color = color;
        self
    }
}

impl From<Folders> for FoldersTable {
    fn from(folders: Folders) -> Self {
        Self {
            folders,
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for FoldersTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new("NAME"), Cell::new("DESC")]))
            .add_rows(
                self.folders
                    .iter()
                    .map(|folder| folder.to_row(&self.config)),
            );

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for FoldersTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.folders.serialize(serializer)
    }
}

/// Represents the printable account.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Account {
    /// Represents the account name.
    pub name: String,
    /// Represents the backend name of the account.
    pub backend: String,
    /// Represents the default state of the account.
    pub default: bool,
}

impl Account {
    pub fn new(name: &str, backend: &str, default: bool) -> Self {
        Self {
            name: name.into(),
            backend: backend.into(),
            default,
        }
    }

    pub fn to_row(&self, config: &ListAccountsTableConfig) -> Row {
        let mut row = Row::new();
        row.max_height(1);

        row.add_cell(Cell::new(&self.name).fg(config.name_color()));
        row.add_cell(Cell::new(&self.backend).fg(config.backends_color()));
        row.add_cell(Cell::new(if self.default { "yes" } else { "" }).fg(config.default_color()));

        row
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents the list of printable accounts.
#[derive(Debug, Default, Serialize)]
pub struct Accounts(Vec<Account>);

impl Deref for Accounts {
    type Target = Vec<Account>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Iter<'_, String, HimalayaTomlAccountConfig>> for Accounts {
    fn from(map: Iter<'_, String, HimalayaTomlAccountConfig>) -> Self {
        let mut accounts: Vec<_> = map
            .map(|(name, account)| {
                #[allow(unused_mut)]
                let mut backends = String::new();

                if let Some(backend) = &account.backend {
                    backends.push_str(&backend.to_string());
                }

                if let Some(backend) = account.message_send_backend() {
                    if !backends.is_empty() {
                        backends.push_str(", ")
                    }
                    backends.push_str(&backend.to_string());
                }

                Account::new(name, &backends, account.default.unwrap_or_default())
            })
            .collect();

        // sort accounts by name
        accounts.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

        Self(accounts)
    }
}

pub struct AccountsTable {
    accounts: Accounts,
    width: Option<u16>,
    config: ListAccountsTableConfig,
}

impl AccountsTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_name_color(mut self, color: Option<Color>) -> Self {
        self.config.name_color = color;
        self
    }

    pub fn with_some_backends_color(mut self, color: Option<Color>) -> Self {
        self.config.backends_color = color;
        self
    }

    pub fn with_some_default_color(mut self, color: Option<Color>) -> Self {
        self.config.default_color = color;
        self
    }
}

impl From<Accounts> for AccountsTable {
    fn from(accounts: Accounts) -> Self {
        Self {
            accounts,
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for AccountsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("NAME"),
                Cell::new("BACKENDS"),
                Cell::new("DEFAULT"),
            ]))
            .add_rows(
                self.accounts
                    .iter()
                    .map(|account| account.to_row(&self.config)),
            );

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for AccountsTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.accounts.serialize(serializer)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Mailbox {
    pub name: Option<String>,
    pub addr: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelope {
    pub id: String,
    pub flags: Flags,
    pub subject: String,
    pub from: Mailbox,
    pub to: Mailbox,
    pub date: String,
    pub has_attachment: bool,
}

impl Envelope {
    fn to_row(&self, config: &ListEnvelopesTableConfig) -> Row {
        let mut all_attributes = vec![];

        let unseen = !self.flags.contains(&Flag::Seen);
        if unseen {
            all_attributes.push(Attribute::Bold)
        }

        let flags = {
            let mut flags = String::new();

            flags.push(config.flagged_char(self.flags.contains(&Flag::Flagged)));
            flags.push(config.unseen_char(unseen));
            flags.push(config.attachment_char(self.has_attachment));
            flags.push(config.replied_char(self.flags.contains(&Flag::Answered)));

            flags
        };

        let mut row = Row::new();
        row.max_height(1);

        row.add_cell(
            Cell::new(&self.id)
                .add_attributes(all_attributes.clone())
                .fg(config.id_color()),
        )
        .add_cell(
            Cell::new(flags)
                .add_attributes(all_attributes.clone())
                .fg(config.flags_color()),
        )
        .add_cell(
            Cell::new(&self.subject)
                .add_attributes(all_attributes.clone())
                .fg(config.subject_color()),
        )
        .add_cell(
            Cell::new(if let Some(name) = &self.from.name {
                name
            } else {
                &self.from.addr
            })
            .add_attributes(all_attributes.clone())
            .fg(config.sender_color()),
        )
        .add_cell(
            Cell::new(&self.date)
                .add_attributes(all_attributes)
                .fg(config.date_color()),
        );

        row
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Envelopes(Vec<Envelope>);

impl Envelopes {
    pub fn try_from_backend(
        config: &AccountConfig,
        id_mapper: &IdMapper,
        envelopes: email::envelope::Envelopes,
    ) -> Result<Envelopes> {
        let envelopes = envelopes
            .iter()
            .map(|envelope| {
                Ok(Envelope {
                    id: id_mapper.get_or_create_alias(&envelope.id)?,
                    flags: envelope.flags.clone().into(),
                    subject: envelope.subject.clone(),
                    from: Mailbox {
                        name: envelope.from.name.clone(),
                        addr: envelope.from.addr.clone(),
                    },
                    to: Mailbox {
                        name: envelope.to.name.clone(),
                        addr: envelope.to.addr.clone(),
                    },
                    date: envelope.format_date(config),
                    has_attachment: envelope.has_attachment,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Envelopes(envelopes))
    }
}

impl Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct EnvelopesTable {
    envelopes: Envelopes,
    width: Option<u16>,
    config: ListEnvelopesTableConfig,
}

impl EnvelopesTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_unseen_char(mut self, char: Option<char>) -> Self {
        self.config.unseen_char = char;
        self
    }

    pub fn with_some_replied_char(mut self, char: Option<char>) -> Self {
        self.config.replied_char = char;
        self
    }

    pub fn with_some_flagged_char(mut self, char: Option<char>) -> Self {
        self.config.flagged_char = char;
        self
    }

    pub fn with_some_attachment_char(mut self, char: Option<char>) -> Self {
        self.config.attachment_char = char;
        self
    }

    pub fn with_some_id_color(mut self, color: Option<Color>) -> Self {
        self.config.id_color = color;
        self
    }

    pub fn with_some_flags_color(mut self, color: Option<Color>) -> Self {
        self.config.flags_color = color;
        self
    }

    pub fn with_some_subject_color(mut self, color: Option<Color>) -> Self {
        self.config.subject_color = color;
        self
    }

    pub fn with_some_sender_color(mut self, color: Option<Color>) -> Self {
        self.config.sender_color = color;
        self
    }

    pub fn with_some_date_color(mut self, color: Option<Color>) -> Self {
        self.config.date_color = color;
        self
    }
}

impl From<Envelopes> for EnvelopesTable {
    fn from(envelopes: Envelopes) -> Self {
        Self {
            envelopes,
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("FLAGS"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]))
            .add_rows(self.envelopes.iter().map(|env| env.to_row(&self.config)));

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for EnvelopesTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.envelopes.serialize(serializer)
    }
}

pub struct ThreadedEnvelopes(email::envelope::ThreadedEnvelopes);

impl ThreadedEnvelopes {
    pub fn try_from_backend(
        id_mapper: &IdMapper,
        envelopes: email::envelope::ThreadedEnvelopes,
    ) -> Result<ThreadedEnvelopes> {
        let prev_edges = envelopes
            .graph()
            .all_edges()
            .map(|(a, b, w)| {
                let a = id_mapper.get_or_create_alias(&a.id)?;
                let b = id_mapper.get_or_create_alias(&b.id)?;
                Ok((a, b, *w))
            })
            .collect::<Result<Vec<_>>>()?;

        let envelopes = envelopes
            .map()
            .iter()
            .map(|(_, envelope)| {
                let id = id_mapper.get_or_create_alias(&envelope.id)?;
                let envelope = email::envelope::Envelope {
                    id: id.clone(),
                    message_id: envelope.message_id.clone(),
                    in_reply_to: envelope.in_reply_to.clone(),
                    flags: envelope.flags.clone(),
                    subject: envelope.subject.clone(),
                    from: envelope.from.clone(),
                    to: envelope.to.clone(),
                    date: envelope.date.clone(),
                    has_attachment: envelope.has_attachment,
                };

                Ok((id, envelope))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let envelopes = email::envelope::ThreadedEnvelopes::build(envelopes, move |envelopes| {
            let mut graph = DiGraphMap::<ThreadedEnvelope, u8>::new();

            for (a, b, w) in prev_edges.clone() {
                let eb = envelopes.get(&b).unwrap();
                match envelopes.get(&a) {
                    Some(ea) => {
                        graph.add_edge(ea.as_threaded(), eb.as_threaded(), w);
                    }
                    None => {
                        let ea = ThreadedEnvelope {
                            id: "0",
                            message_id: "0",
                            subject: "",
                            from: "",
                            date: Default::default(),
                        };
                        graph.add_edge(ea, eb.as_threaded(), w);
                    }
                }
            }

            graph
        });

        Ok(ThreadedEnvelopes(envelopes))
    }
}

impl Deref for ThreadedEnvelopes {
    type Target = email::envelope::ThreadedEnvelopes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct EnvelopesTree {
    config: Arc<AccountConfig>,
    envelopes: ThreadedEnvelopes,
}

impl EnvelopesTree {
    pub fn new(config: Arc<AccountConfig>, envelopes: ThreadedEnvelopes) -> Self {
        Self { config, envelopes }
    }

    pub fn fmt(
        f: &mut fmt::Formatter,
        config: &AccountConfig,
        graph: &DiGraphMap<ThreadedEnvelope<'_>, u8>,
        parent: ThreadedEnvelope<'_>,
        pad: String,
        weight: u8,
    ) -> fmt::Result {
        let edges = graph
            .all_edges()
            .filter_map(|(a, b, w)| {
                if a == parent && *w == weight {
                    Some(b)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if parent.id == "0" {
            f.write_str("root")?;
        } else {
            write!(f, "{}{}", parent.id.red(), ") ".dark_grey())?;

            if !parent.subject.is_empty() {
                write!(f, "{} ", parent.subject.green())?;
            }

            if !parent.from.is_empty() {
                let left = "<".dark_grey();
                let right = ">".dark_grey();
                write!(f, "{left}{}{right}", parent.from.blue())?;
            }

            let date = parent.format_date(config);
            let cursor_date_begin_col = terminal::size().unwrap().0 - date.len() as u16;

            let dots =
                "·".repeat((cursor_date_begin_col - cursor::position().unwrap().0 - 2) as usize);
            write!(f, " {} {}", dots.dark_grey(), date.dark_yellow())?;
        }

        writeln!(f)?;

        let edges_count = edges.len();
        for (i, b) in edges.into_iter().enumerate() {
            let is_last = edges_count == i + 1;
            let (x, y) = if is_last {
                (' ', '└')
            } else {
                ('│', '├')
            };

            write!(f, "{pad}{y}─ ")?;

            let pad = format!("{pad}{x}  ");
            Self::fmt(f, config, graph, b, pad, weight + 1)?;
        }

        Ok(())
    }
}

impl fmt::Display for EnvelopesTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        EnvelopesTree::fmt(
            f,
            &self.config,
            self.envelopes.0.graph(),
            ThreadedEnvelope {
                id: "0",
                message_id: "0",
                from: "",
                subject: "",
                date: Default::default(),
            },
            String::new(),
            0,
        )
    }
}

impl Serialize for EnvelopesTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.envelopes.0.serialize(serializer)
    }
}

impl Deref for EnvelopesTree {
    type Target = ThreadedEnvelopes;

    fn deref(&self) -> &Self::Target {
        &self.envelopes
    }
}

/// Represents the flag variants.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Custom(String),
}

impl From<&email::flag::Flag> for Flag {
    fn from(flag: &email::flag::Flag) -> Self {
        use email::flag::Flag::*;
        match flag {
            Seen => Flag::Seen,
            Answered => Flag::Answered,
            Flagged => Flag::Flagged,
            Deleted => Flag::Deleted,
            Draft => Flag::Draft,
            Custom(flag) => Flag::Custom(flag.clone()),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Flags(pub HashSet<Flag>);

impl Deref for Flags {
    type Target = HashSet<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<email::flag::Flags> for Flags {
    fn from(flags: email::flag::Flags) -> Self {
        Flags(flags.iter().map(Flag::from).collect())
    }
}
