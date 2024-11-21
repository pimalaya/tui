use std::fmt;
#[cfg(feature = "path")]
use std::path::{Path, PathBuf};

use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};

use crate::{terminal::validator::*, Error, Result};

pub fn u16(prompt: impl AsRef<str>, default: Option<u16>) -> Result<u16> {
    let prompt = Text::new(prompt.as_ref()).with_validator(U16Validator);

    let number = if let Some(default) = default {
        prompt.with_default(&default.to_string()).prompt()
    } else {
        prompt.prompt()
    };

    match number {
        Ok(number) => Ok(number.parse().unwrap()),
        Err(err) => Err(Error::PromptU16Error(err)),
    }
}

pub fn usize(prompt: impl AsRef<str>, default: Option<usize>) -> Result<usize> {
    let prompt = Text::new(prompt.as_ref()).with_validator(UsizeValidator);

    let number = if let Some(default) = default {
        prompt.with_default(&default.to_string()).prompt()
    } else {
        prompt.prompt()
    };

    match number {
        Ok(number) => Ok(number.parse().unwrap()),
        Err(err) => Err(Error::PromptUsizeError(err)),
    }
}

pub fn secret(prompt: impl AsRef<str>) -> Result<String> {
    Password::new(prompt.as_ref())
        .with_display_mode(PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()
        .map_err(Error::PromptSecretError)
}

pub fn some_secret(prompt: impl AsRef<str>) -> Result<Option<String>> {
    Password::new(prompt.as_ref())
        .with_display_mode(PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt_skippable()
        .map_err(Error::PromptSecretError)
}

pub fn password(prompt: impl AsRef<str>) -> Result<String> {
    Password::new(prompt.as_ref())
        .with_display_mode(PasswordDisplayMode::Masked)
        .with_custom_confirmation_message("Confirm password")
        .prompt()
        .map_err(Error::PromptPasswordError)
}

pub fn text<T: AsRef<str>>(prompt: T, default: Option<T>) -> Result<String> {
    let mut prompt = Text::new(prompt.as_ref());

    if let Some(default) = default.as_ref() {
        prompt = prompt.with_default(default.as_ref())
    }

    prompt.prompt().map_err(Error::PromptTextError)
}

pub fn some_text<T: AsRef<str>>(prompt: T, default: Option<T>) -> Result<Option<String>> {
    let mut prompt = Text::new(prompt.as_ref());

    if let Some(default) = default.as_ref() {
        prompt = prompt.with_default(default.as_ref())
    }

    prompt.prompt_skippable().map_err(Error::PromptTextError)
}

pub fn bool(prompt: impl AsRef<str>, default: bool) -> Result<bool> {
    Confirm::new(prompt.as_ref())
        .with_default(default)
        .prompt()
        .map_err(Error::PromptBoolError)
}

pub fn item<T: fmt::Display + Eq>(
    prompt: impl AsRef<str>,
    items: impl IntoIterator<Item = T>,
    default: Option<T>,
) -> Result<T> {
    let items: Vec<_> = items.into_iter().collect();

    let default = if let Some(default) = default.as_ref() {
        items
            .iter()
            .enumerate()
            .find_map(|(i, item)| if item == default { Some(i) } else { None })
    } else {
        None
    };

    let mut prompt = Select::new(prompt.as_ref(), items);

    if let Some(default) = default.as_ref() {
        prompt = prompt.with_starting_cursor(*default);
    }

    prompt.prompt().map_err(Error::PromptItemError)
}

#[cfg(feature = "path")]
pub fn path(prompt: impl AsRef<str>, default: Option<impl AsRef<Path>>) -> Result<PathBuf> {
    let prompt = Text::new(prompt.as_ref());

    let text = if let Some(default) = default.as_ref() {
        let default = default.as_ref().display().to_string();
        prompt.with_default(&default).prompt()
    } else {
        prompt.prompt()
    };

    let path = PathBuf::from(text.map_err(Error::PromptPathError)?);

    Ok(shellexpand_utils::expand::path(path))
}

#[cfg(feature = "email")]
pub fn email<T: AsRef<str>>(prompt: T, default: Option<T>) -> Result<email_address::EmailAddress> {
    let mut prompt = Text::new(prompt.as_ref()).with_validator(EmailValidator);

    if let Some(default) = default.as_ref() {
        prompt = prompt.with_default(default.as_ref());
    }

    let email = prompt.prompt().map_err(Error::PromptEmailError)?;

    Ok(<email_address::EmailAddress as std::str::FromStr>::from_str(&email).unwrap())
}
