use inquire::{
    validator::{StringValidator, Validation},
    CustomUserError,
};

#[derive(Clone, Debug, Default)]
pub struct U16Validator;

impl StringValidator for U16Validator {
    fn validate(&self, input: &str) -> Result<Validation, CustomUserError> {
        match input.parse::<u16>() {
            Ok(_) => Ok(Validation::Valid),
            Err(err) => Err(Box::new(err)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UsizeValidator;

impl StringValidator for UsizeValidator {
    fn validate(&self, input: &str) -> Result<Validation, CustomUserError> {
        match input.parse::<usize>() {
            Ok(_) => Ok(Validation::Valid),
            Err(err) => Err(Box::new(err)),
        }
    }
}

#[cfg(feature = "email")]
#[derive(Clone, Debug, Default)]
pub struct EmailValidator;

#[cfg(feature = "email")]
impl StringValidator for EmailValidator {
    fn validate(&self, input: &str) -> Result<Validation, CustomUserError> {
        match <email_address::EmailAddress as std::str::FromStr>::from_str(input) {
            Ok(_) => Ok(Validation::Valid),
            Err(err) => Err(Box::new(err)),
        }
    }
}
