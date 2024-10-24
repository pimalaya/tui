use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use color_eyre::Result;
#[cfg(feature = "imap")]
use email::imap::{ImapContext, ImapContextBuilder};
#[cfg(feature = "maildir")]
use email::maildir::{MaildirContextBuilder, MaildirContextSync};
#[cfg(feature = "notmuch")]
use email::notmuch::{NotmuchContextBuilder, NotmuchContextSync};
#[cfg(feature = "sendmail")]
use email::sendmail::{SendmailContextBuilder, SendmailContextSync};
#[cfg(feature = "smtp")]
use email::smtp::{SmtpContextBuilder, SmtpContextSync};
use email::{
    account::config::AccountConfig,
    backend::{
        context::BackendContextBuilder, feature::BackendFeature, macros::BackendContext,
        mapper::SomeBackendContextBuilderMapper,
    },
    envelope::{
        list::{ListEnvelopes, ListEnvelopesOptions},
        thread::ThreadEnvelopes,
        Id, SingleId,
    },
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags, Flag, Flags},
    folder::list::ListFolders,
    message::{
        add::AddMessage,
        copy::CopyMessages,
        delete::DeleteMessages,
        get::GetMessages,
        peek::PeekMessages,
        r#move::MoveMessages,
        send::{SendMessage, SendMessageThenSaveCopy},
        Messages,
    },
    AnyResult,
};

use super::{
    config::{self, Envelopes, HimalayaTomlAccountConfig, ThreadedEnvelopes},
    id_mapper::IdMapper,
};

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
    pub backend: Option<config::Backend>,
    pub sending_backend: Option<config::SendingBackend>,

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

impl ContextBuilder {
    pub fn new(
        toml_account_config: Arc<HimalayaTomlAccountConfig>,
        account_config: Arc<AccountConfig>,
    ) -> Self {
        Self {
            backend: toml_account_config.backend.clone(),
            sending_backend: toml_account_config
                .message
                .as_ref()
                .and_then(|c| c.send.as_ref())
                .and_then(|c| c.backend.clone()),

            #[cfg(feature = "imap")]
            imap: toml_account_config.backend.as_ref().and_then(|backend| {
                #[allow(irrefutable_let_patterns)]
                let config::Backend::Imap(imap) = backend
                else {
                    return None;
                };

                Some(ImapContextBuilder::new(
                    account_config.clone(),
                    Arc::new(imap.clone()),
                ))
            }),
            #[cfg(feature = "maildir")]
            maildir: toml_account_config.backend.as_ref().and_then(|backend| {
                #[allow(irrefutable_let_patterns)]
                let config::Backend::Maildir(maildir) = backend
                else {
                    return None;
                };

                Some(MaildirContextBuilder::new(
                    account_config.clone(),
                    Arc::new(maildir.clone()),
                ))
            }),
            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config.backend.as_ref().and_then(|backend| {
                #[allow(irrefutable_let_patterns)]
                let config::Backend::Notmuch(notmuch) = backend
                else {
                    return None;
                };

                Some(NotmuchContextBuilder::new(
                    account_config.clone(),
                    Arc::new(notmuch.clone()),
                ))
            }),
            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .message
                .as_ref()
                .and_then(|msg| msg.send.as_ref())
                .and_then(|send| send.backend.as_ref())
                .and_then(|backend| {
                    #[allow(irrefutable_let_patterns)]
                    let config::SendingBackend::Smtp(smtp) = backend
                    else {
                        return None;
                    };

                    Some(SmtpContextBuilder::new(
                        account_config.clone(),
                        Arc::new(smtp.clone()),
                    ))
                }),
            #[cfg(feature = "sendmail")]
            sendmail: toml_account_config
                .message
                .as_ref()
                .and_then(|msg| msg.send.as_ref())
                .and_then(|send| send.backend.as_ref())
                .and_then(|backend| {
                    #[allow(irrefutable_let_patterns)]
                    let config::SendingBackend::Sendmail(sendmail) = backend
                    else {
                        return None;
                    };

                    Some(SendmailContextBuilder::new(
                        account_config.clone(),
                        Arc::new(sendmail.clone()),
                    ))
                }),
        }
    }
}

#[async_trait]
impl BackendContextBuilder for ContextBuilder {
    type Context = Context;

    fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.list_folders_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.list_folders_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.list_folders_with_some(&self.notmuch),
        }
    }

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.list_envelopes_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.list_envelopes_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.list_envelopes_with_some(&self.notmuch),
        }
    }

    fn get_messages(&self) -> Option<BackendFeature<Self::Context, dyn GetMessages>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.get_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.get_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.get_messages_with_some(&self.notmuch),
        }
    }

    fn add_message(&self) -> Option<BackendFeature<Self::Context, dyn AddMessage>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.add_message_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.add_message_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.add_message_with_some(&self.notmuch),
        }
    }

    fn send_message(&self) -> Option<BackendFeature<Self::Context, dyn SendMessage>> {
        match self.sending_backend.as_ref()? {
            config::SendingBackend::None => None,
            #[cfg(feature = "smtp")]
            config::SendingBackend::Smtp(_) => self.send_message_with_some(&self.smtp),
            #[cfg(feature = "sendmail")]
            config::SendingBackend::Sendmail(_) => self.send_message_with_some(&self.sendmail),
        }
    }

    fn copy_messages(&self) -> Option<BackendFeature<Self::Context, dyn CopyMessages>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.copy_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.copy_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.copy_messages_with_some(&self.notmuch),
        }
    }

    fn move_messages(&self) -> Option<BackendFeature<Self::Context, dyn MoveMessages>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.move_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.move_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.move_messages_with_some(&self.notmuch),
        }
    }

    fn delete_messages(&self) -> Option<BackendFeature<Self::Context, dyn DeleteMessages>> {
        match self.backend.as_ref()? {
            config::Backend::None => None,
            #[cfg(feature = "imap")]
            config::Backend::Imap(_) => self.delete_messages_with_some(&self.imap),
            #[cfg(feature = "maildir")]
            config::Backend::Maildir(_) => self.delete_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            config::Backend::Notmuch(_) => self.delete_messages_with_some(&self.notmuch),
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

pub struct Backend {
    toml_account_config: Arc<HimalayaTomlAccountConfig>,
    backend: email::backend::Backend<Context>,
}

impl Backend {
    fn build_id_mapper(&self, folder: &str, backend: Option<&config::Backend>) -> Result<IdMapper> {
        #[cfg(all(feature = "maildir", feature = "sled"))]
        if let Some(config::Backend::Maildir(_)) = backend {
            return Ok(IdMapper::new(&self.backend.account_config, folder)?);
        }

        #[cfg(all(feature = "notmuch", feature = "sled"))]
        if let Some(config::Backend::Notmuch(_)) = backend {
            return Ok(IdMapper::new(&self.backend.account_config, folder)?);
        }

        Ok(IdMapper::Dummy)
    }

    pub async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> Result<Envelopes> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let envelopes = self.backend.list_envelopes(folder, opts).await?;
        let envelopes =
            Envelopes::try_from_backend(&self.backend.account_config, &id_mapper, envelopes)?;
        Ok(envelopes)
    }

    pub async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> Result<ThreadedEnvelopes> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let envelopes = self.backend.thread_envelopes(folder, opts).await?;
        let envelopes = ThreadedEnvelopes::try_from_backend(&id_mapper, envelopes)?;
        Ok(envelopes)
    }

    pub async fn thread_envelope(
        &self,
        folder: &str,
        id: usize,
        opts: ListEnvelopesOptions,
    ) -> Result<ThreadedEnvelopes> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = id_mapper.get_id(id)?;
        let envelopes = self
            .backend
            .thread_envelope(folder, SingleId::from(id), opts)
            .await?;
        let envelopes = ThreadedEnvelopes::try_from_backend(&id_mapper, envelopes)?;
        Ok(envelopes)
    }

    pub async fn add_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flags(folder, &ids, flags).await?;
        Ok(())
    }

    pub async fn add_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flag(folder, &ids, flag).await?;
        Ok(())
    }

    pub async fn set_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flags(folder, &ids, flags).await?;
        Ok(())
    }

    pub async fn set_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flag(folder, &ids, flag).await?;
        Ok(())
    }

    pub async fn remove_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flags(folder, &ids, flags).await?;
        Ok(())
    }

    pub async fn remove_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flag(folder, &ids, flag).await?;
        Ok(())
    }

    pub async fn add_message(&self, folder: &str, email: &[u8]) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self.backend.add_message(folder, email).await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    pub async fn add_message_with_flags(
        &self,
        folder: &str,
        email: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self
            .backend
            .add_message_with_flags(folder, email, flags)
            .await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    pub async fn peek_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        let msgs = self.backend.peek_messages(folder, &ids).await?;
        Ok(msgs)
    }

    pub async fn get_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        let msgs = self.backend.get_messages(folder, &ids).await?;
        Ok(msgs)
    }

    pub async fn copy_messages(
        &self,
        from_folder: &str,
        to_folder: &str,
        ids: &[usize],
    ) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(from_folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend
            .copy_messages(from_folder, to_folder, &ids)
            .await?;
        Ok(())
    }

    pub async fn move_messages(
        &self,
        from_folder: &str,
        to_folder: &str,
        ids: &[usize],
    ) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(from_folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend
            .move_messages(from_folder, to_folder, &ids)
            .await?;
        Ok(())
    }

    pub async fn delete_messages(&self, folder: &str, ids: &[usize]) -> Result<()> {
        let backend_kind = self.toml_account_config.backend.as_ref();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.delete_messages(folder, &ids).await?;
        Ok(())
    }

    pub async fn send_message_then_save_copy(&self, msg: &[u8]) -> Result<()> {
        self.backend.send_message_then_save_copy(msg).await?;
        Ok(())
    }
}

pub struct BackendBuilder {
    toml_account_config: Arc<HimalayaTomlAccountConfig>,
    builder: email::backend::BackendBuilder<ContextBuilder>,
}

impl BackendBuilder {
    pub fn new(
        toml_account_config: Arc<HimalayaTomlAccountConfig>,
        account_config: Arc<AccountConfig>,
        f: impl Fn(
            email::backend::BackendBuilder<ContextBuilder>,
        ) -> email::backend::BackendBuilder<ContextBuilder>,
    ) -> BackendBuilder {
        let builder = email::backend::BackendBuilder::new(
            account_config.clone(),
            ContextBuilder::new(toml_account_config.clone(), account_config),
        );

        Self {
            toml_account_config,
            builder: f(builder),
        }
    }

    pub fn without_backend(mut self) -> Self {
        #[cfg(feature = "imap")]
        {
            self.builder.ctx_builder.imap = None;
        }
        #[cfg(feature = "maildir")]
        {
            self.builder.ctx_builder.maildir = None;
        }
        #[cfg(feature = "notmuch")]
        {
            self.builder.ctx_builder.notmuch = None;
        }

        self
    }

    pub fn without_sending_backend(mut self) -> Self {
        #[cfg(feature = "smtp")]
        {
            self.builder.ctx_builder.smtp = None;
        }
        #[cfg(feature = "sendmail")]
        {
            self.builder.ctx_builder.sendmail = None;
        }

        self
    }

    pub async fn build(self) -> Result<Backend> {
        Ok(Backend {
            toml_account_config: self.toml_account_config,
            backend: self.builder.build().await?,
        })
    }
}

impl Deref for Backend {
    type Target = email::backend::Backend<Context>;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}
