use std::{collections::HashMap, error::Error, sync::Arc};

use axum::{
    Json, Router, debug_handler,
    extract::{Path, Query, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header::LOCATION},
    response::{Html, IntoResponse},
    routing::get,
};
use chrono::Days;
use jsonwebtoken::{EncodingKey, Header, encode};
use maud::{Markup, Render};
use serde::{Deserialize, Serialize};
use serenity::all::User;
use sqlx::SqlitePool;
use strum::EnumString;

use crate::DISCORD_URL;

const AUTH_TOKEN_EXPIRATION: Days = Days::new(7);
const DISCORD_SIGN_UP_TOKEN_EXPIRATION: Days = Days::new(1);

#[derive(Clone, Copy, Debug, EnumString, Deserialize)]
enum ApiResponseKind {
    #[serde(rename = "json")]
    #[strum(serialize = "json")]
    Json,
    #[serde(rename = "html")]
    #[strum(serialize = "html")]
    Html,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BasicJsonResponse {
    message: String,
}

trait IntoApiSuccess<J, H>
where
    J: Serialize,
    H: IntoResponse,
{
    fn into_json(self) -> (J, Option<StatusCode>, Vec<(HeaderName, HeaderValue)>);
    fn into_html(self) -> (H, Option<StatusCode>, Vec<(HeaderName, HeaderValue)>);

    fn into_api_success(self, response_kind: ApiResponseKind) -> ApiSuccess<J, H>
    where
        Self: Sized,
    {
        match response_kind {
            ApiResponseKind::Json => {
                let (json, status, headers) = self.into_json();
                ApiSuccess::Json((json, status.unwrap_or(StatusCode::OK), headers))
            }
            ApiResponseKind::Html => {
                let (html, status, headers) = self.into_html();
                ApiSuccess::Html((html, status.unwrap_or(StatusCode::OK), headers))
            }
        }
    }
}

trait IntoApiFailure<J, H>
where
    J: Serialize,
    H: IntoResponse,
{
    fn into_json(self) -> (J, Option<StatusCode>, Vec<(HeaderName, HeaderValue)>);
    fn into_html(self) -> (H, Option<StatusCode>, Vec<(HeaderName, HeaderValue)>);

    fn into_api_failure(self, response_kind: ApiResponseKind) -> ApiFailure<J, H>
    where
        Self: Sized,
    {
        match response_kind {
            ApiResponseKind::Json => {
                let (json, status, headers) = self.into_json();
                ApiFailure::Json((json, status.unwrap_or(StatusCode::BAD_REQUEST), headers))
            }
            ApiResponseKind::Html => {
                let (html, status, headers) = self.into_html();
                ApiFailure::Html((html, status.unwrap_or(StatusCode::BAD_REQUEST), headers))
            }
        }
    }
}

enum ApiSuccess<J, H>
where
    J: Serialize,
    H: IntoResponse,
{
    Json((J, StatusCode, Vec<(HeaderName, HeaderValue)>)),
    Html((H, StatusCode, Vec<(HeaderName, HeaderValue)>)),
}

enum ApiFailure<J, H>
where
    J: Serialize,
    H: IntoResponse,
{
    Json((J, StatusCode, Vec<(HeaderName, HeaderValue)>)),
    Html((H, StatusCode, Vec<(HeaderName, HeaderValue)>)),
}

type ApiResult<Js, Hs, Jf, Hf> = Result<ApiSuccess<Js, Hs>, ApiFailure<Jf, Hf>>;

impl<J, H> IntoResponse for ApiSuccess<J, H>
where
    J: Serialize,
    H: IntoResponse,
{
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Json((json, code, headers)) => {
                let mut response = Json::from(json).into_response();
                *response.status_mut() = code;
                response.headers_mut().extend(headers);
                response
            }
            Self::Html((html, code, headers)) => {
                let mut response = Html::from(html).into_response();
                *response.status_mut() = code;
                response.headers_mut().extend(headers);
                response
            }
        }
    }
}

impl<J, H> IntoResponse for ApiFailure<J, H>
where
    J: Serialize,
    H: IntoResponse,
{
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Json((json, code, headers)) => {
                let mut response = Json::from(json).into_response();
                *response.status_mut() = code;
                response.headers_mut().extend(headers);
                response
            }
            Self::Html((html, code, headers)) => {
                let mut response = Html::from(html).into_response();
                *response.status_mut() = code;
                response.headers_mut().extend(headers);
                response
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct JsonError {
    message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authorization code missing")]
    AuthorizationCodeMissing,
    #[error("discord api request failed")]
    DiscordApiFailure(#[from] reqwest::Error),
    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),
}

impl IntoApiFailure<JsonError, Markup> for ApiError {
    fn into_json(
        self,
    ) -> (
        JsonError,
        Option<StatusCode>,
        Vec<(HeaderName, HeaderValue)>,
    ) {
        (
            JsonError {
                message: format!("{self}"),
            },
            None,
            vec![],
        )
    }

    fn into_html(self) -> (Markup, Option<StatusCode>, Vec<(HeaderName, HeaderValue)>) {
        (format!("{self}").render(), None, vec![])
    }
}

// #[derive(Debug, thiserror::Error, Serialize)]
// pub enum HtmlApiError {
//     #[error("authorization code missing")]
//     AuthorizationCodeMissing,
// }
//
// #[derive(Debug, thiserror::Error, Serialize)]
// pub enum JsonApiError {
//     #[error("authorization code missing")]
//     AuthorizationCodeMissing,
// }
//
// impl IntoResponse for HtmlApiError {
//     fn into_response(self) -> axum::response::Response {
//         match self {
//             Self::AuthorizationCodeMissing => {
//                 let mut response = html! { "authorization code missing" }.into_response();
//                 *response.status_mut() = StatusCode::BAD_REQUEST;
//                 response
//             }
//         }
//     }
// }
//
// impl IntoResponse for JsonApiError {
//     fn into_response(self) -> axum::response::Response {
//         #[derive(Debug, Serialize)]
//         struct ErrorJson {
//             message: String,
//         }
//
//         match self {
//             Self::AuthorizationCodeMissing => {
//                 let mut response = Json::from(ErrorJson {
//                     message: "authorization code missing".to_string(),
//                 })
//                 .into_response();
//                 *response.status_mut() = StatusCode::BAD_REQUEST;
//                 response
//             }
//         }
//     }
// }

#[derive(Clone)]
pub struct ApiState {
    pub server_address: Arc<str>,
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
            self.message.render(),
            Some(StatusCode::FOUND),
            vec![
                (LOCATION, HeaderValue::from_str(&self.next_location)
                    .expect("the location header value should not have non-visible ascii characters; we are creating it!")
                )
            ]
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
        token_type: String,
        expires_in: u32,
        refresh_token: String,
        scope: String,
    }

    let response = reqwest::Client::new()
        .post(format!("{DISCORD_URL}/api/oauth2/token"))
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
        .get(format!("{DISCORD_URL}/api/users/@me"))
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

    if let Some(record) = sqlx::query!(
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
        Ok(AuthorizeDiscordResponse {
            message: "signed in :3".to_string(),
            next_location: state.server_address.to_string(),
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
        .into_api_success(kind))
    } else {
        Ok(AuthorizeDiscordResponse {
            message: "continue!".to_string(),
            next_location: format!("{}/sign-up/discord", state.server_address),
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
        .into_api_success(kind))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordSignUpTokenClaims {
    exp: usize,
    sub: i64,
    username: String,
    avatar_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthTokenClaims {
    exp: usize,
    sub: i64,
}
