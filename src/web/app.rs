use std::sync::Arc;

use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::extract::cookie::Cookie;
use http::{HeaderValue, StatusCode, header::SET_COOKIE};
use maud::{Markup, html};
use serde::Deserialize;
use thiserror::Error;
use time::{Duration, SignedDuration};

use crate::{
    APP_PATH,
    auth::{Auth, AuthError, HOURS_PER_DAY, TOKEN_EXPIRATION_DAYS},
    database::{DatabaseError, User, UserId},
    discord::{DiscordError, DiscordUser},
    web::WebState,
};

const DISCORD_AUTH_PATH: &str = "/discord-auth";

#[derive(Debug, Error)]
pub enum AppError {
    // TODO we need to be careful not to be so leaky with error messages
    #[error("discord error: {0}")]
    Discord(#[from] DiscordError),
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("auth error: {0}")]
    Auth(#[from] AuthError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, html! { (self) }).into_response()
        // match self {
        //     _ => (StatusCode::BAD_REQUEST, html! { (self) }).into_response(),
        // }
    }
}

pub fn routes() -> Router<Arc<WebState>> {
    Router::new()
        .route("/", get(index))
        .route(DISCORD_AUTH_PATH, get(discord_auth))
}

fn discord_auth_url(server_url: &str) -> String {
    format!("{}{APP_PATH}{DISCORD_AUTH_PATH}", server_url)
}

async fn index(State(state): State<Arc<WebState>>) -> Markup {
    html! {
        a href=(
            &state
                .config
                .discord_client
                .get_authorization_url(
                    &discord_auth_url(&state.config.server_url), None
                )
        ) { "discord auth" }
    }
}

#[derive(Debug, Deserialize)]
struct DiscordAuthParams {
    code: String,
}

async fn discord_auth(
    State(state): State<Arc<WebState>>,
    Query(params): Query<DiscordAuthParams>,
) -> Result<Response, AppError> {
    let token = state
        .config
        .discord_client
        .exchange_code_for_token(&params.code, &discord_auth_url(&state.config.server_url))
        .await?;

    let discord_user = DiscordUser::get(&token).await?;
    let user = match User::fetch(UserId::DiscordId(&discord_user.id), &state.config.pool).await? {
        Some(user) => user,
        None => {
            User::create_with_discord_id(
                &state.config.snowflake_manager,
                &discord_user.username,
                &discord_user.id,
                &state.config.pool,
            )
            .await?
        }
    };

    let (token, max_age) = state.config.auth.generate_user_token(&user)?;
    let cookie = Cookie::build(("token", token))
        .path("/")
        // TODO unify Duration across project
        .max_age(SignedDuration::milliseconds(max_age.as_millis() as i64))
        .build();
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string())
            .expect("token header value should not be invalid"),
    );

    Ok(response)
}
