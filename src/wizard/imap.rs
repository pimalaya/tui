#[cfg(feature = "oauth2")]
use email::{
    account::config::oauth2::{OAuth2Config, OAuth2Method, OAuth2Scopes},
    autoconfig::config::AuthenticationType,
};
use email::{
    account::config::passwd::PasswdConfig,
    autoconfig::config::{AutoConfig, SecurityType, ServerType},
    imap::config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
};
use email_address::EmailAddress;
#[cfg(feature = "oauth2")]
use oauth::v2_0::{AuthorizationCodeGrant, Client};
use secret::Secret;

use crate::{prompt, Result};

static ENCRYPTIONS: [ImapEncryptionKind; 3] = [
    ImapEncryptionKind::Tls,
    ImapEncryptionKind::StartTls,
    ImapEncryptionKind::None,
];

static SECRETS: &[&str] = &[
    #[cfg(feature = "keyring")]
    KEYRING,
    RAW,
    CMD,
];

#[cfg(feature = "keyring")]
const KEYRING: &str = "Ask my password, then save it in my system's global keyring";
const RAW: &str = "Ask my password, then save it in the configuration file (not safe)";
const CMD: &str = "Ask me a shell command that exposes my password";

pub async fn start(
    account_name: impl AsRef<str>,
    email: &EmailAddress,
    autoconfig: Option<&AutoConfig>,
) -> Result<ImapConfig> {
    let account_name = account_name.as_ref();

    let autoconfig_server = autoconfig.and_then(|c| {
        c.email_provider()
            .incoming_servers()
            .into_iter()
            .find(|server| matches!(server.server_type(), ServerType::Imap))
    });

    let autoconfig_host = autoconfig_server
        .and_then(|s| s.hostname())
        .map(ToOwned::to_owned);

    let default_host = autoconfig_host.unwrap_or_else(|| format!("imap.{}", email.domain()));

    let host = prompt::text("IMAP hostname:", Some(&default_host))?;

    let autoconfig_encryption = autoconfig_server
        .and_then(|imap| {
            imap.security_type().map(|encryption| match encryption {
                SecurityType::Plain => ImapEncryptionKind::None,
                SecurityType::Starttls => ImapEncryptionKind::StartTls,
                SecurityType::Tls => ImapEncryptionKind::Tls,
            })
        })
        .unwrap_or_default();

    let autoconfig_port = autoconfig_server
        .and_then(|config| config.port())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| match &autoconfig_encryption {
            ImapEncryptionKind::Tls => 993,
            ImapEncryptionKind::StartTls => 143,
            ImapEncryptionKind::None => 143,
        });

    let encryption = prompt::item(
        "IMAP encryption:",
        ENCRYPTIONS.clone(),
        Some(autoconfig_encryption.clone()),
    )?;

    let default_port = match encryption {
        ref encryption if encryption == &autoconfig_encryption => autoconfig_port,
        ImapEncryptionKind::Tls => 993,
        ImapEncryptionKind::StartTls => 143,
        ImapEncryptionKind::None => 143,
    };

    let port = prompt::u16("IMAP port:", Some(default_port))?;

    let autoconfig_login = autoconfig_server.map(|imap| match imap.username() {
        Some("%EMAILLOCALPART%") => email.local_part().to_owned(),
        Some("%EMAILADDRESS%") => email.to_string(),
        _ => email.to_string(),
    });

    let default_login = autoconfig_login.unwrap_or_else(|| email.to_string());

    let login = prompt::text("IMAP login:", Some(&default_login))?;

    // ------------

    #[cfg(feature = "oauth2")]
    let auth = {
        const OAUTH2_MECHANISMS: [OAuth2Method; 2] =
            [OAuth2Method::XOAuth2, OAuth2Method::OAuthBearer];

        let autoconfig_oauth2 = autoconfig.and_then(|c| c.oauth2());

        let default_oauth2_enabled = autoconfig_server
            .and_then(|imap| {
                imap.authentication_type()
                    .into_iter()
                    .find_map(|t| Option::from(matches!(t, AuthenticationType::OAuth2)))
            })
            .filter(|_| autoconfig_oauth2.is_some())
            .unwrap_or_default();

        let oauth2_enabled = prompt::bool("Enable OAuth 2.0?", default_oauth2_enabled)?;

        if oauth2_enabled {
            let mut config = OAuth2Config::default();
            let redirect_host = OAuth2Config::LOCALHOST;
            let redirect_port = OAuth2Config::get_first_available_port()?;

            config.method = prompt::item(
                "IMAP OAuth 2.0 mechanism:",
                OAUTH2_MECHANISMS.clone(),
                Some(OAuth2Method::XOAuth2),
            )?;

            config.client_id = prompt::text("IMAP OAuth 2.0 client id:", None)?;

            let client_secret = prompt::secret("IMAP OAuth 2.0 client secret:")?;
            config.client_secret =
                Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-client-secret"))?;
            config
                .client_secret
                .set_only_keyring(&client_secret)
                .await?;

            let default_auth_url = autoconfig_oauth2
                .map(|config| config.auth_url().to_owned())
                .unwrap_or_default();
            config.auth_url =
                prompt::text("IMAP OAuth 2.0 authorization URL:", Some(&default_auth_url))?;

            let default_token_url = autoconfig_oauth2
                .map(|config| config.token_url().to_owned())
                .unwrap_or_default();
            config.token_url = prompt::text("IMAP OAuth 2.0 token URL:", Some(&default_token_url))?;

            let autoconfig_scopes = autoconfig_oauth2.map(|config| config.scope());

            let prompt_scope = |prompt: &str| -> Result<Option<String>> {
                Ok(match &autoconfig_scopes {
                    Some(scopes) => Some(prompt::item(prompt, scopes.to_vec(), None)?.to_string()),
                    None => Some(prompt::text(prompt, None)?).filter(|scope| !scope.is_empty()),
                })
            };

            if let Some(scope) = prompt_scope("IMAP OAuth 2.0 main scope:")? {
                config.scopes = OAuth2Scopes::Scope(scope);
            }

            let confirm_additional_scope = || -> Result<bool> {
                let confirm = prompt::bool("More IMAP OAuth 2.0 scopes?", false)?;
                Ok(confirm)
            };

            while confirm_additional_scope()? {
                let mut scopes = match config.scopes {
                    OAuth2Scopes::Scope(scope) => vec![scope],
                    OAuth2Scopes::Scopes(scopes) => scopes,
                };

                if let Some(scope) = prompt_scope("Additional IMAP OAuth 2.0 scope:")? {
                    scopes.push(scope)
                }

                config.scopes = OAuth2Scopes::Scopes(scopes);
            }

            config.pkce = prompt::bool("Enable PKCE verification?", true)?;

            crate::print::section("To complete your OAuth 2.0 setup, click on the following link:");

            let client = Client::new(
                config.client_id.clone(),
                client_secret,
                config.auth_url.clone(),
                config.token_url.clone(),
            )?
            .with_redirect_host(redirect_host.to_owned())
            .with_redirect_port(redirect_port)
            .build()?;

            let mut auth_code_grant = AuthorizationCodeGrant::new()
                .with_redirect_host(redirect_host.to_owned())
                .with_redirect_port(redirect_port);

            if config.pkce {
                auth_code_grant = auth_code_grant.with_pkce();
            }

            for scope in config.scopes.clone() {
                auth_code_grant = auth_code_grant.with_scope(scope);
            }

            let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

            println!("{redirect_url}");
            println!();

            let (access_token, refresh_token) = auth_code_grant
                .wait_for_redirection(&client, csrf_token)
                .await?;

            config.access_token =
                Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-access-token"))?;
            config.access_token.set_only_keyring(access_token).await?;

            if let Some(refresh_token) = &refresh_token {
                config.refresh_token = Secret::try_new_keyring_entry(format!(
                    "{account_name}-imap-oauth2-refresh-token"
                ))?;
                config.refresh_token.set_only_keyring(refresh_token).await?;
            }

            ImapAuthConfig::OAuth2(config)
        } else {
            configure_passwd(account_name).await?
        }
    };

    #[cfg(not(feature = "oauth2"))]
    let auth = configure_passwd(account_name).await?;

    Ok(ImapConfig {
        host,
        port,
        encryption: Some(encryption),
        login,
        auth,
        watch: None,
        extensions: None,
        clients_pool_size: None,
    })
}

pub(crate) async fn configure_passwd(account_name: &str) -> Result<ImapAuthConfig> {
    let secret = match prompt::item("IMAP authentication strategy:", SECRETS, None)? {
        #[cfg(feature = "keyring")]
        &KEYRING => {
            let secret = Secret::try_new_keyring_entry(format!("{account_name}-imap-passwd"))?;
            secret
                .set_only_keyring(prompt::password("IMAP password:")?)
                .await?;
            secret
        }
        &RAW => Secret::new_raw(prompt::password("IMAP password:")?),
        &CMD => Secret::new_command(prompt::text(
            "Shell command:",
            Some(&format!("pass show {account_name}")),
        )?),
        _ => unreachable!(),
    };

    Ok(ImapAuthConfig::Passwd(PasswdConfig(secret)))
}
