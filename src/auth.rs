use std::time::{Duration, SystemTime, UNIX_EPOCH};

use grand_music_cup::current_millis;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::database::User;

pub const TOKEN_EXPIRATION_DAYS: u64 = 7;
pub const HOURS_PER_DAY: u64 = 24;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("jwt error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error("token expired")]
    TokenExpired,
}

pub struct Auth {
    secret: Box<[u8]>,
}

impl Auth {
    pub fn new(secret: Box<[u8]>) -> Self {
        Self {
            secret: Box::from(secret),
        }
    }

    pub fn generate_user_token(&self, user: &User) -> Result<(String, Duration), AuthError> {
        let expiration = Duration::from_hours(TOKEN_EXPIRATION_DAYS * HOURS_PER_DAY);
        let claims = UserClaims {
            sub: user.id(),
            exp: (current_millis() + expiration.as_millis()) as u64,
        };
        Ok((
            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(&self.secret),
            )?,
            expiration,
        ))
    }

    pub fn authenticate_user_token(&self, token: &str) -> Result<i64, AuthError> {
        let claims = decode::<UserClaims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )?
        .claims;
        if (claims.exp as u128) < current_millis() {
            Err(AuthError::TokenExpired)
        } else {
            Ok(claims.sub)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserClaims {
    sub: i64,
    exp: u64,
}
