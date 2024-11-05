use std::{fs, path::PathBuf};

use async_trait::async_trait;
use dirs::{config_dir, home_dir};
use serde::Deserialize;
use serde_toml_merge::merge;
use toml::Value;

use crate::{Error, Result};

#[async_trait]
pub trait TomlConfig: for<'de> Deserialize<'de> {
    type TomlAccountConfig;

    fn project_name() -> &'static str;

    fn get_default_account_config(&self) -> Option<(String, Self::TomlAccountConfig)>;
    fn get_account_config(&self, name: &str) -> Option<(String, Self::TomlAccountConfig)>;

    #[cfg(feature = "wizard")]
    async fn from_wizard(path: &std::path::Path) -> color_eyre::Result<Self>;

    /// Read and parse the TOML configuration at the given paths
    ///
    /// Returns an error if a configuration file cannot be read or if
    /// a content cannot be parsed.
    fn from_paths(paths: &[PathBuf]) -> Result<Self> {
        match paths.len() {
            0 => {
                return Err(Error::ReadTomlConfigFileFromEmptyPaths);
            }
            1 => {
                let path = &paths[0];

                let ref content = fs::read_to_string(path)
                    .map_err(|err| Error::ReadTomlConfigFile(err, path.clone()))?;

                toml::from_str(content).map_err(|err| Error::ParseTomlConfigFile(err, path.clone()))
            }
            _ => {
                let path = &paths[0];

                let mut merged_content = fs::read_to_string(path)
                    .map_err(|err| Error::ReadTomlConfigFile(err, path.clone()))?
                    .parse::<Value>()
                    .map_err(|err| Error::ParseTomlConfigFile(err, path.clone()))?;

                for path in &paths[1..] {
                    let content = fs::read_to_string(path);

                    #[cfg(feature = "tracing")]
                    if let Err(err) = &content {
                        tracing::debug!(?path, ?err, "skipping invalid subconfig file");
                    }

                    let Ok(content) = content else {
                        continue;
                    };

                    let content = content
                        .parse()
                        .map_err(|err| Error::ParseTomlConfigFile(err, path.clone()))?;

                    merged_content =
                        merge(merged_content, content).map_err(Error::MergeTomlConfigFiles)?;
                }

                merged_content
                    .try_into()
                    .map_err(|err| Error::ParseTomlConfigFile(err, path.clone()))
            }
        }
    }

    /// Read and parse the TOML configuration at the optional given
    /// path.
    ///
    /// If the given path exists, then read and parse the TOML
    /// configuration from it.
    ///
    /// If the given path does not exist, then create it using the
    /// wizard.
    ///
    /// If no path is given, then either read and parse the TOML
    /// configuration at the first valid default path, otherwise
    /// create it using the wizard.  wizard.
    async fn from_paths_or_default(paths: &[PathBuf]) -> Result<Self> {
        match paths.len() {
            0 => Self::from_default_paths().await,
            _ if paths[0].exists() => Self::from_paths(paths),
            #[cfg(feature = "wizard")]
            _ => Self::from_wizard(&paths[0])
                .await
                .map_err(Error::CreateTomlConfigFromWizardError),
            #[cfg(not(feature = "wizard"))]
            _ => Err(Error::CreateTomlConfigFromInvalidPathsError),
        }
    }

    /// Read and parse the TOML configuration from default paths.
    async fn from_default_paths() -> Result<Self> {
        match Self::first_valid_default_path() {
            Some(path) => Self::from_paths(&[path]),
            #[cfg(feature = "wizard")]
            None => Self::from_wizard(&Self::default_path()?)
                .await
                .map_err(Error::CreateTomlConfigFromWizardError),
            #[cfg(not(feature = "wizard"))]
            None => Err(Error::CreateTomlConfigFromInvalidPathsError),
        }
    }

    /// Get the default configuration path
    ///
    /// Returns an error if the XDG configuration directory cannot be
    /// found.
    fn default_path() -> Result<PathBuf> {
        let Some(dir) = config_dir() else {
            return Err(Error::GetXdgConfigDirectory);
        };

        Ok(dir.join(Self::project_name()).join("config.toml"))
    }

    /// Get the first default configuration path that points to a
    /// valid file
    ///
    /// Tries paths in this order:
    ///
    /// - `$XDG_CONFIG_DIR/<project>/config.toml`
    /// - `$HOME/.config/<project>/config.toml`
    /// - `$HOME/.<project>rc`
    fn first_valid_default_path() -> Option<PathBuf> {
        let project = Self::project_name();

        Self::default_path()
            .ok()
            .filter(|p| p.exists())
            .or_else(|| home_dir().map(|p| p.join(".config").join(project).join("config.toml")))
            .filter(|p| p.exists())
            .or_else(|| home_dir().map(|p| p.join(format!(".{project}rc"))))
            .filter(|p| p.exists())
    }

    #[cfg(feature = "wizard")]
    fn set_table_dotted(table: &mut toml_edit::Table) {
        let keys: Vec<String> = table.iter().map(|(key, _)| key.to_string()).collect();

        for ref key in keys {
            if let Some(table) = table.get_mut(key).unwrap().as_table_mut() {
                table.set_dotted(true);
                Self::set_table_dotted(table)
            }
        }
    }

    #[cfg(feature = "wizard")]
    fn write(&self, path: &std::path::Path) -> Result<()>
    where
        Self: serde::Serialize,
    {
        use crate::terminal::prompt;

        let path = prompt::path("Where to save the configuration?", Some(path))?;
        println!("Writing configuration at {}…", path.display());

        let mut doc: toml_edit::DocumentMut = toml::to_string(&self)
            .map_err(Error::SerializeTomlConfigError)?
            .parse()
            .map_err(Error::ParseSerializedTomlConfigError)?;

        doc.iter_mut().for_each(|(_, item)| {
            if let Some(table) = item.as_table_mut() {
                table.iter_mut().for_each(|(_, item)| {
                    if let Some(table) = item.as_table_mut() {
                        Self::set_table_dotted(table);
                    }
                })
            }
        });

        fs::create_dir_all(path.parent().unwrap_or(&path))
            .map_err(|err| Error::CreateTomlConfigParentDirectoryError(err, path.clone()))?;
        fs::write(&path, doc.to_string())
            .map_err(|err| Error::WriteTomlConfigError(err, path.clone()))?;

        println!("Done! Exiting the wizard…");
        Ok(())
    }

    fn to_toml_account_config(
        &self,
        account_name: Option<&str>,
    ) -> Result<(String, Self::TomlAccountConfig)> {
        match account_name {
            Some("default") | Some("") | None => self
                .get_default_account_config()
                .ok_or(Error::GetDefaultAccountConfigError),
            Some(name) => self
                .get_account_config(name)
                .ok_or_else(|| Error::GetAccountConfigError(name.to_owned())),
        }
    }

    fn into_account_configs<C, A>(
        self,
        account_name: Option<&str>,
        get_account: impl Fn(C) -> Option<A>,
    ) -> Result<(Self::TomlAccountConfig, A)>
    where
        Self: Into<C>,
    {
        let (account_name, toml_account_config) = self.to_toml_account_config(account_name)?;

        let account_config =
            get_account(self.into()).ok_or_else(|| Error::BuildAccountConfigError(account_name))?;

        Ok((toml_account_config, account_config))
    }
}
