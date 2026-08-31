use argon2::password_hash::{self};
use thiserror::Error;

use crate::snowflake::SnowflakeError;

pub mod cup;
pub mod user;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("sql error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("password hash error: {0}")]
    PasswordHash(password_hash::Error),
    #[error("error making snowflake id: {0}")]
    Snowflake(#[from] SnowflakeError),
}

impl From<password_hash::Error> for ModelError {
    fn from(value: password_hash::Error) -> Self {
        ModelError::PasswordHash(value)
    }
}

fn i64_to_bool(value: i64) -> bool {
    value != 0
}
