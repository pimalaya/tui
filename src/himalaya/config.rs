use std::{collections::HashMap, fmt, path::PathBuf};

use comfy_table::presets;
use crossterm::style::Color;
#[cfg(feature = "pgp")]
use email::account::config::pgp::PgpConfig;
#[cfg(feature = "imap")]
use email::imap::config::ImapConfig;
#[cfg(feature = "maildir")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "notmuch")]
use email::notmuch::config::NotmuchConfig;
#[cfg(feature = "sendmail")]
use email::sendmail::config::SendmailConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpConfig;
use email::{
    account::config::AccountConfig,
    config::Config,
    message::{
        add::config::MessageWriteConfig, delete::config::DeleteMessageConfig,
        get::config::MessageReadConfig,
    },
    template::config::TemplateConfig,
};
use process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case", deny_unknown_fields)
)]
pub struct HimalayaTomlConfig {
    #[cfg_attr(feature = "config", serde(alias = "name"))]
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

#[cfg(feature = "config")]
#[async_trait::async_trait]
impl crate::terminal::config::TomlConfig for HimalayaTomlConfig {
    type AccountConfig = HimalayaTomlAccountConfig;

    fn project_name() -> &'static str {
        "himalaya"
    }

    fn get_default_account_config(&self) -> Option<(String, Self::AccountConfig)> {
        self.accounts.iter().find_map(|(name, account)| {
            account
                .default
                .filter(|default| *default)
                .map(|_| (name.to_owned(), account.clone()))
        })
    }

    fn get_account_config(&self, name: &str) -> Option<(String, Self::AccountConfig)> {
        self.accounts
            .get(name)
            .map(|account| (name.to_owned(), account.clone()))
    }

    #[cfg(feature = "wizard")]
    async fn from_wizard(path: &std::path::Path) -> crate::Result<Self> {
        super::wizard::confirm_or_exit(path)?;
        let config = super::wizard::run(path).await?;

        Ok(config)
    }

    fn to_toml_account_config(
        &self,
        account_name: Option<&str>,
    ) -> crate::Result<(String, Self::AccountConfig)> {
        let (name, mut config) = match account_name {
            Some("default") | Some("") | None => self
                .get_default_account_config()
                .ok_or(crate::Error::GetDefaultAccountConfigError),
            Some(name) => self
                .get_account_config(name)
                .ok_or_else(|| crate::Error::GetAccountConfigError(name.to_owned())),
        }?;

        #[cfg(all(feature = "imap", feature = "keyring"))]
        if let Some(imap_config) = config.imap.as_mut() {
            imap_config.auth.replace_undefined_keyring_entries(&name)?;
        }

        #[cfg(all(feature = "smtp", feature = "keyring"))]
        if let Some(smtp_config) = config.smtp.as_mut() {
            smtp_config.auth.replace_undefined_keyring_entries(&name)?;
        }

        Ok((name, config))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case", deny_unknown_fields)
)]
pub struct HimalayaTomlAccountConfig {
    pub default: Option<bool>,
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub backend: Option<BackendKind>,

    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,

    pub folder: Option<FolderConfig>,
    pub envelope: Option<EnvelopeConfig>,
    pub message: Option<MessageConfig>,
    pub template: Option<TemplateConfig>,

    #[cfg(feature = "imap")]
    pub imap: Option<ImapConfig>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirConfig>,
    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchConfig>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpConfig>,
    #[cfg(feature = "sendmail")]
    pub sendmail: Option<SendmailConfig>,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct AccountsConfig {
    pub list: Option<ListAccountsConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ListAccountsConfig {
    pub table: Option<ListAccountsTableConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum BackendKind {
    #[default]
    None,
    #[cfg(feature = "imap")]
    Imap,
    #[cfg(feature = "maildir")]
    Maildir,
    #[cfg(feature = "notmuch")]
    Notmuch,
    #[cfg(feature = "smtp")]
    Smtp,
    #[cfg(feature = "sendmail")]
    Sendmail,
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
                #[cfg(feature = "smtp")]
                Self::Smtp => "SMTP",
                #[cfg(feature = "sendmail")]
                Self::Sendmail => "Sendmail",
            }
        )
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "config",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct SendMessageConfig {
    pub backend: Option<BackendKind>,
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
