use std::time::Duration;

use grand_music_cup::current_millis;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::database::{User, UserId};

pub const TOKEN_EXPIRATION_DAYS: u64 = 7;
pub const HOURS_PER_DAY: u64 = 24;

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("jwt error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error("token expired")]
    TokenExpired,
}

pub fn generate_user_token(user: &User, secret: &[u8]) -> Result<(String, Duration), TokenError> {
    let expiration = Duration::from_hours(TOKEN_EXPIRATION_DAYS * HOURS_PER_DAY);
    let claims = UserClaims {
        sub: user.id(),
        exp: (current_millis() + expiration.as_millis()) as u64,
    };
    Ok((
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )?,
        expiration,
    ))
}

pub fn authenticate_user_token(token: &str, secret: &[u8]) -> Result<UserId<'static>, TokenError> {
    let claims = decode::<UserClaims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )?
    .claims;
    if (claims.exp as u128) < current_millis() {
        Err(TokenError::TokenExpired)
    } else {
        Ok(UserId::PrimaryKey(claims.sub))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UserClaims {
    sub: i64,
    exp: u64,
}
