use argon2::{
    Argon2, PasswordHasher,
    password_hash::{self, Salt},
};
use base64::prelude::*;
use rand::rngs::StdRng;
use serde::Deserialize;
use sqlx::SqlitePool;
use thiserror::Error;

use crate::{crypto::random_bytes, snowflake::SnowflakeManager};

const SALT_LEN: usize = 16;

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct User {
    id: i64,
    display_name: String,
    discord_id: Option<String>,
    login_name: Option<String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct UserWithPassword {
    id: i64,
    display_name: String,
    discord_id: Option<String>,
    login_name: Option<String>,
    password_hash: Option<String>,
    salt: Option<String>,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("sql error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("password hash error: {0}")]
    PasswordHashError(password_hash::Error),
}

impl From<password_hash::Error> for DatabaseError {
    fn from(value: password_hash::Error) -> Self {
        DatabaseError::PasswordHashError(value)
    }
}

pub enum UserId<'a> {
    PrimaryKey(i64),
    LoginName(&'a str),
    DiscordId(&'a str),
}

impl User {
    pub async fn fetch(id: UserId<'_>, pool: &SqlitePool) -> Result<Option<Self>, DatabaseError> {
        Ok(match id {
            UserId::PrimaryKey(k) => {
                sqlx::query_as!(
                    Self,
                    "SELECT id, display_name, discord_id, login_name FROM users WHERE id = ?",
                    k
                )
                .fetch_optional(pool)
                .await?
            }
            UserId::LoginName(l) => sqlx::query_as!(
                Self,
                "SELECT id, display_name, discord_id, login_name FROM users WHERE login_name = ?",
                l
            )
            .fetch_optional(pool)
            .await?,
            UserId::DiscordId(d) => sqlx::query_as!(
                Self,
                "SELECT id, display_name, discord_id, login_name FROM users WHERE discord_id = ?",
                d
            )
            .fetch_optional(pool)
            .await?,
        })
    }

    pub async fn exists(id: UserId<'_>, pool: &SqlitePool) -> Result<bool, DatabaseError> {
        Ok(match id {
            UserId::PrimaryKey(k) => {
                sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)", k)
                    .fetch_one(pool)
                    .await
            }
            UserId::LoginName(l) => {
                sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users WHERE login_name = ?)", l)
                    .fetch_one(pool)
                    .await
            }
            UserId::DiscordId(d) => {
                sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users WHERE discord_id = ?)", d)
                    .fetch_one(pool)
                    .await
            }
        }
        .map(|b| b != 0)?)
    }

    pub async fn create_with_discord_id(
        snowflake_manager: &SnowflakeManager,
        display_name: &str,
        discord_id: &str,
        pool: &SqlitePool,
    ) -> Result<Self, DatabaseError> {
        Ok(sqlx::query_as!(
            Self,
            "
        INSERT INTO users (id, display_name, discord_id)
        VALUES (?, ?, ?)
        RETURNING id, display_name, discord_id, login_name
        ",
            snowflake_manager.make_snowflake(),
            display_name,
            discord_id
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn create_with_login_name(
        snowflake_manager: &SnowflakeManager,
        display_name: &str,
        login_name: &str,
        password: &str,
        rng: &mut StdRng,
        pool: &SqlitePool,
    ) -> Result<Self, DatabaseError> {
        let salt = BASE64_STANDARD.encode(random_bytes::<SALT_LEN>(rng));
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), Salt::from_b64(&salt)?)?
            .to_string();
        Ok(sqlx::query_as!(
            Self,
            "
        INSERT INTO users (id, display_name, login_name, password_hash, salt)
        VALUES (?, ?, ?, ?, ?)
        RETURNING id, display_name, discord_id, login_name
        ",
            snowflake_manager.make_snowflake(),
            display_name,
            login_name,
            password_hash,
            salt,
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn authenticate(
        login_name: &str,
        password: &str,
        pool: &SqlitePool,
    ) -> Result<Option<Self>, DatabaseError> {
        let Some(user) = UserWithPassword::fetch(login_name, pool).await? else {
            return Ok(None);
        };

        if user
            .password_hash
            .as_ref()
            .expect("login name should always have associated password hash")
            == Argon2::default()
                .hash_password(
                    password.as_bytes(),
                    Salt::from_b64(
                        user.salt
                            .as_ref()
                            .expect("login name should always have associated salt"),
                    )?,
                )?
                .to_string()
                .as_str()
        {
            Ok(Some(user.into()))
        } else {
            Ok(None)
        }
    }
}

impl UserWithPassword {
    async fn fetch(login_name: &str, pool: &SqlitePool) -> Result<Option<Self>, DatabaseError> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM users WHERE login_name = ?", login_name)
                .fetch_optional(pool)
                .await?,
        )
    }
}

impl From<UserWithPassword> for User {
    fn from(value: UserWithPassword) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            discord_id: value.discord_id,
            login_name: value.login_name,
        }
    }
}
