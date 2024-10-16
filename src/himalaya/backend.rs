use async_trait::async_trait;
#[cfg(feature = "imap")]
use email::imap::{config::ImapConfig, ImapContext, ImapContextBuilder};
#[cfg(feature = "maildir")]
use email::maildir::{config::MaildirConfig, MaildirContextBuilder, MaildirContextSync};
#[cfg(feature = "notmuch")]
use email::notmuch::{config::NotmuchConfig, NotmuchContextBuilder, NotmuchContextSync};
#[cfg(feature = "sendmail")]
use email::sendmail::{config::SendmailConfig, SendmailContextBuilder, SendmailContextSync};
#[cfg(feature = "smtp")]
use email::smtp::{config::SmtpConfig, SmtpContextBuilder, SmtpContextSync};
use email::{
    backend::{
        context::BackendContextBuilder, feature::BackendFeature, macros::BackendContext,
        mapper::SomeBackendContextBuilderMapper,
    },
    envelope::list::ListEnvelopes,
    folder::list::ListFolders,
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        r#move::MoveMessages, send::SendMessage,
    },
    AnyResult,
};

use super::config::BackendKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendConfig {
    #[cfg(feature = "imap")]
    Imap(ImapConfig),
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(feature = "notmuch")]
    Notmuch(NotmuchConfig),
    #[cfg(feature = "smtp")]
    Smtp(SmtpConfig),
    #[cfg(feature = "sendmail")]
    Sendmail(SendmailConfig),
}

#[derive(BackendContext)]
pub struct Context {
    #[cfg(feature = "imap")]
    imap: Option<ImapContext>,
    #[cfg(feature = "maildir")]
    maildir: Option<MaildirContextSync>,
    #[cfg(feature = "notmuch")]
    notmuch: Option<NotmuchContextSync>,
    #[cfg(feature = "smtp")]
    smtp: Option<SmtpContextSync>,
    #[cfg(feature = "sendmail")]
    sendmail: Option<SendmailContextSync>,
}

#[cfg(feature = "imap")]
impl AsRef<Option<ImapContext>> for Context {
    fn as_ref(&self) -> &Option<ImapContext> {
        &self.imap
    }
}

#[cfg(feature = "maildir")]
impl AsRef<Option<MaildirContextSync>> for Context {
    fn as_ref(&self) -> &Option<MaildirContextSync> {
        &self.maildir
    }
}

#[cfg(feature = "notmuch")]
impl AsRef<Option<NotmuchContextSync>> for Context {
    fn as_ref(&self) -> &Option<NotmuchContextSync> {
        &self.notmuch
    }
}

#[cfg(feature = "smtp")]
impl AsRef<Option<SmtpContextSync>> for Context {
    fn as_ref(&self) -> &Option<SmtpContextSync> {
        &self.smtp
    }
}

#[cfg(feature = "sendmail")]
impl AsRef<Option<SendmailContextSync>> for Context {
    fn as_ref(&self) -> &Option<SendmailContextSync> {
        &self.sendmail
    }
}

#[derive(Clone)]
pub struct ContextBuilder {
    pub backend: BackendKind,
    pub sending_backend: BackendKind,

    #[cfg(feature = "imap")]
    pub imap: Option<ImapContextBuilder>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirContextBuilder>,
    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchContextBuilder>,
    #[cfg(feature = "sendmail")]
    pub sendmail: Option<SendmailContextBuilder>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpContextBuilder>,
}

#[async_trait]
impl BackendContextBuilder for ContextBuilder {
    type Context = Context;

    fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.list_folders_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.list_folders_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.list_folders_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.list_envelopes_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.list_envelopes_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.list_envelopes_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn get_messages(&self) -> Option<BackendFeature<Self::Context, dyn GetMessages>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.get_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.get_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.get_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn add_message(&self) -> Option<BackendFeature<Self::Context, dyn AddMessage>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.add_message_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.add_message_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.add_message_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn send_message(&self) -> Option<BackendFeature<Self::Context, dyn SendMessage>> {
        match self.sending_backend {
            #[cfg(feature = "smtp")]
            BackendKind::Smtp => self.send_message_with_some(&self.smtp),
            #[cfg(feature = "sendmail")]
            BackendKind::Sendmail => self.send_message_with_some(&self.sendmail),
            _ => None,
        }
    }

    fn copy_messages(&self) -> Option<BackendFeature<Self::Context, dyn CopyMessages>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.copy_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.copy_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.copy_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn move_messages(&self) -> Option<BackendFeature<Self::Context, dyn MoveMessages>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.move_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.move_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.move_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn delete_messages(&self) -> Option<BackendFeature<Self::Context, dyn DeleteMessages>> {
        match self.backend {
            #[cfg(feature = "imap")]
            BackendKind::Imap => self.delete_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            BackendKind::Maildir => self.delete_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            BackendKind::Notmuch => self.delete_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    async fn build(self) -> AnyResult<Self::Context> {
        #[cfg(feature = "imap")]
        let imap = match self.imap {
            Some(imap) => Some(imap.build().await?),
            None => None,
        };

        #[cfg(feature = "maildir")]
        let maildir = match self.maildir {
            Some(maildir) => Some(maildir.build().await?),
            None => None,
        };

        #[cfg(feature = "notmuch")]
        let notmuch = match self.notmuch {
            Some(notmuch) => Some(notmuch.build().await?),
            None => None,
        };

        #[cfg(feature = "smtp")]
        let smtp = match self.smtp {
            Some(smtp) => Some(smtp.build().await?),
            None => None,
        };

        #[cfg(feature = "sendmail")]
        let sendmail = match self.sendmail {
            Some(sendmail) => Some(sendmail.build().await?),
            None => None,
        };

        Ok(Context {
            #[cfg(feature = "imap")]
            imap,
            #[cfg(feature = "maildir")]
            maildir,
            #[cfg(feature = "notmuch")]
            notmuch,
            #[cfg(feature = "smtp")]
            smtp,
            #[cfg(feature = "sendmail")]
            sendmail,
        })
    }
}

pub type Backend = email::backend::Backend<Context>;
