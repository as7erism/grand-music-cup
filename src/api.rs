use std::{collections::HashMap, error::Error, sync::Arc};

use axum::{
    Json, Router, debug_handler,
    extract::{Path, Query, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header::LOCATION},
    response::{Html, IntoResponse},
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::Days;
use jsonwebtoken::{EncodingKey, Header, encode};
use maud::{Markup, Render, html};
use reqwest::header::{ACCESS_CONTROL_ALLOW_ORIGIN, SET_COOKIE};
use serde::{Deserialize, Serialize};
use serenity::all::User;
use sqlx::SqlitePool;
use strum::EnumString;

use crate::{AuthTokenClaims, DISCORD_URI, DiscordSignUpTokenClaims};

mod html;
mod json;
mod union;

const AUTH_TOKEN_EXPIRATION: Days = Days::new(7);
const DISCORD_SIGN_UP_TOKEN_EXPIRATION: Days = Days::new(1);

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authorization code missing")]
    AuthorizationCodeMissing,
    #[error("discord api request failed")]
    DiscordApiFailure(#[from] reqwest::Error),
    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Clone, Serialize)]
struct ErrorAsJson {
    message: String,
}

impl ApiError {
    fn into_json(self) -> (StatusCode, Json<ErrorAsJson>) {
        match self {
            _ => (StatusCode::BAD_REQUEST, Json::from(ErrorAsJson { message: format!{"{self}"} }))
        }
    }

    fn into_html(self) -> (StatusCode, Html<Markup>) {
        match self {
            _ => (StatusCode::BAD_REQUEST, Html::from(html! { (self) }))
        }
    }
}

#[derive(Clone)]
pub struct ApiState {
    pub server_uri: Arc<str>,
    pub discord_authorization_callback_url: Arc<str>,
    pub discord_client_id: Arc<str>,
    pub discord_client_secret: Arc<str>,
    pub jwt_secret: Arc<[u8]>,
    pub pool: SqlitePool,
}

pub fn init_api() -> Router<ApiState> {
    Router::new().route("/{kind}/authorize/discord", get(authorize_discord))
}

#[derive(Debug, Serialize)]
struct AuthorizeDiscordResponse {
    message: String,
    next_location: String,
    token: String,
}

impl IntoApiSuccess<AuthorizeDiscordResponse, Markup> for AuthorizeDiscordResponse {
    fn into_json(
        self,
    ) -> (
        AuthorizeDiscordResponse,
        Option<StatusCode>,
        Vec<(HeaderName, HeaderValue)>,
    ) {
        (self, None, vec![])
    }

    fn into_html(self) -> (Markup, Option<StatusCode>, Vec<(HeaderName, HeaderValue)>) {
        (
            html! {
                html {
                    head {
                        meta http-equiv="refresh" content="0; url='http://127.0.0.1:8464/sign-up/discord" {}
                    }
                    body {}
                }
            },
            // self.message.render(),
            Some(StatusCode::OK),
            vec![
                (
                    SET_COOKIE,
                    HeaderValue::from_str(
                        &Cookie::build(("token", &self.token))
                            .same_site(SameSite::Strict)
                            .path("/")
                            .build()
                            .to_string(),
                    )
                    .inspect(|c| println!("{c:?}"))
                    .unwrap(),
                    // HeaderValue::from_str(&format!("token={}", self.token))
                    //     .expect("the token should not have non-visible ascii characters"),
                ),
            ],
        )
    }
}

#[debug_handler]
async fn authorize_discord(
    Path(kind): Path<ApiResponseKind>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<ApiState>,
    _request: Request,
) -> ApiResult<AuthorizeDiscordResponse, Markup, JsonError, Markup> {
    let code = params
        .get("code")
        .ok_or(ApiError::AuthorizationCodeMissing.into_api_failure(kind))?;

    #[derive(Deserialize)]
    struct AccessTokenResponse {
        access_token: String,
    }

    let response = reqwest::Client::new()
        .post(format!("{DISCORD_URI}/api/oauth2/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &state.discord_authorization_callback_url),
        ])
        .basic_auth(state.discord_client_id, Some(state.discord_client_secret))
        .send()
        .await
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| ApiError::from(e).into_api_failure(kind))?
        .error_for_status()
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| ApiError::from(e).into_api_failure(kind))?
        .json::<AccessTokenResponse>()
        .await
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| ApiError::from(e).into_api_failure(kind))?;

    let user = reqwest::Client::new()
        .get(format!("{DISCORD_URI}/api/users/@me"))
        .bearer_auth(response.access_token)
        .send()
        .await
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| ApiError::from(e).into_api_failure(kind))?
        .error_for_status()
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| ApiError::from(e).into_api_failure(kind))?
        .json::<User>()
        .await
        .inspect_err(|e| println!("{e}"))
        .map_err(|e| ApiError::from(e).into_api_failure(kind))?;

    Ok(if let Some(record) = sqlx::query!(
        "
    SELECT id, discord_id
    FROM users
    WHERE discord_id = ?
    ",
        user.id.get() as i64
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::from(e).into_api_failure(kind))?
    {
        // this account has already been set up
        AuthorizeDiscordResponse {
            message: "signed in :3".to_string(),
            next_location: state.server_uri.to_string(),
            token: encode(
                &Header::default(),
                &AuthTokenClaims {
                    exp: (chrono::Utc::now() + AUTH_TOKEN_EXPIRATION).timestamp() as usize,
                    sub: record.id,
                },
                &EncodingKey::from_secret(&state.jwt_secret),
            )
            .expect("im  tired"),
        }
    } else {
        AuthorizeDiscordResponse {
            message: "continue!".to_string(),
            next_location: format!("{}/sign-up/discord", state.server_uri),
            token: encode(
                &Header::default(),
                &DiscordSignUpTokenClaims {
                    exp: (chrono::Utc::now() + DISCORD_SIGN_UP_TOKEN_EXPIRATION).timestamp()
                        as usize,
                    sub: user.id.get() as i64,
                    username: user.name,
                    avatar_hash: user.avatar.map(|h| h.to_string()),
                },
                &EncodingKey::from_secret(&state.jwt_secret),
            )
            .expect("im  tired"),
        }
    }
    .into_api_success(kind))
}
