use argon2::password_hash::{self};
use thiserror::Error;

use crate::snowflake::SnowflakeError;

pub mod cup;
pub mod user;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("sql error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("password hash error: {0}")]
    PasswordHashError(password_hash::Error),
    #[error("error making snowflake id: {0}")]
    SnowflakeError(#[from] SnowflakeError),
}

impl From<password_hash::Error> for ModelError {
    fn from(value: password_hash::Error) -> Self {
        ModelError::PasswordHashError(value)
    }
}
