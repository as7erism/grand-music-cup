use argon2::{Argon2, PasswordHasher};
use grand_music_cup::U10;
use rand::rngs::StdRng;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{crypto::random_bytes, model::ModelError, snowflake::Snowflake};

const SALT_LEN: usize = 16;

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub(super) id: i64,
    pub(super) display_name: String,
    pub(super) discord_id: Option<String>,
    pub(super) login_name: Option<String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
struct UserWithPassword {
    id: i64,
    display_name: String,
    discord_id: Option<String>,
    login_name: Option<String>,
    password_hash: Option<Vec<u8>>,
    salt: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum UserId<'a> {
    PrimaryKey(i64),
    LoginName(&'a str),
    DiscordId(&'a str),
}

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    pub login_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SignUpParams {
    pub login_name: String,
    pub display_name: String,
    pub password: String,
}

impl User {
    pub async fn fetch(id: UserId<'_>, pool: &SqlitePool) -> Result<Option<Self>, ModelError> {
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

    pub async fn exists(id: UserId<'_>, pool: &SqlitePool) -> Result<bool, ModelError> {
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
        display_name: &str,
        discord_id: &str,
        epoch_ms: u64,
        machine_id: U10,
        pool: &SqlitePool,
    ) -> Result<Self, ModelError> {
        Ok(sqlx::query_as!(
            Self,
            "
        INSERT INTO users (id, display_name, discord_id)
        VALUES (?, ?, ?)
        RETURNING id, display_name, discord_id, login_name
        ",
            Snowflake::new_unique(epoch_ms, machine_id)?.as_i64(),
            display_name,
            discord_id
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn create_with_login_name(
        params: &SignUpParams,
        rng: &mut StdRng,
        epoch_ms: u64,
        machine_id: U10,
        pool: &SqlitePool,
    ) -> Result<Self, ModelError> {
        let salt = random_bytes::<SALT_LEN>(rng);
        let password_hash = Argon2::default()
            .hash_password_with_salt(params.password.as_bytes(), &salt)?
            .hash
            .expect("we just hashed this");
        Ok(sqlx::query_as!(
            Self,
            "
        INSERT INTO users (id, display_name, login_name, password_hash, salt)
        VALUES (?, ?, ?, ?, ?)
        RETURNING id, display_name, discord_id, login_name
        ",
            Snowflake::new_unique(epoch_ms, machine_id)?.as_i64(),
            &params.display_name,
            &params.login_name,
            password_hash.as_bytes(),
            salt.as_ref(),
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn authenticate(
        params: &LoginParams,
        pool: &SqlitePool,
    ) -> Result<Option<Self>, ModelError> {
        let Some(user) = UserWithPassword::fetch(&params.login_name, pool).await? else {
            return Ok(None);
        };

        if **user
            .password_hash
            .as_ref()
            .expect("login name should always have associated password hash")
            == *Argon2::default()
                .hash_password_with_salt(
                    params.password.as_bytes(),
                    user.salt
                        .as_ref()
                        .expect("login name should always have associated salt"),
                )?
                .hash
                .expect("we just computed the hash")
                .as_bytes()
        {
            Ok(Some(user.into()))
        } else {
            Ok(None)
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn login_name(&self) -> Option<&str> {
        self.login_name.as_deref()
    }

    pub fn discord_id(&self) -> Option<&str> {
        self.discord_id.as_deref()
    }
}

impl UserWithPassword {
    async fn fetch(login_name: &str, pool: &SqlitePool) -> Result<Option<Self>, ModelError> {
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
