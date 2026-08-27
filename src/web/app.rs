use std::{ops::DerefMut, sync::Arc};

use axum::{
    Form, Router,
    extract::{FromRequestParts, OptionalFromRequestParts, Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use http::{HeaderValue, StatusCode, header::SET_COOKIE};
use maud::{Markup, html};
use rand::rngs::StdRng;
use serde::Deserialize;
use thiserror::Error;
use time::SignedDuration;
use tokio::sync::Mutex;

use crate::{
    APP_PATH,
    config::WebConfig,
    database::{DatabaseError, User, UserId},
    discord::{DiscordError, DiscordUser},
    token::{TokenError, authenticate_user_token, generate_user_token},
    web::app::{AppError::LoggedIn, views::page},
};

mod views;

const DISCORD_AUTH_PATH: &str = "/discord-auth";

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Arc<WebConfig>,
    pub rng: Arc<Mutex<StdRng>>,
}

#[derive(Debug, Error)]
pub enum AppError {
    // TODO we need to be careful not to be so leaky with error messages
    #[error("discord error: {0}")]
    DiscordError(#[from] DiscordError),
    #[error("database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    #[error("unauthenticated")]
    Unauthenticated,
    #[error("already logged in")]
    LoggedIn,
    #[error("token error: {0}")]
    TokenError(#[from] TokenError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::LoggedIn => Redirect::to("/").into_response(),
            _ => (StatusCode::BAD_REQUEST, html! { (self) }).into_response(),
        }
    }
}

impl FromRequestParts<AppState> for User {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies = CookieJar::from_headers(&parts.headers);
        let token = cookies
            .get("token")
            .ok_or(AppError::Unauthenticated)?
            .value();
        let user_id = authenticate_user_token(token, &state.config.jwt_secret)
            .map_err(|_| AppError::Unauthenticated)?;
        User::fetch(user_id, &state.config.pool)
            .await
            .map_err(|_| AppError::Unauthenticated)?
            .ok_or(AppError::Unauthenticated)
    }
}

impl OptionalFromRequestParts<AppState> for User {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        state: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <User as FromRequestParts<AppState>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route(DISCORD_AUTH_PATH, get(discord_auth))
        .route("/log-in", get(get_log_in))
        .route("/log-in", post(post_log_in))
        .route("/sign-up", get(get_sign_up))
        .route("/sign-up", post(post_sign_up))
}

fn discord_auth_url(server_url: &str) -> String {
    format!("{}{APP_PATH}{DISCORD_AUTH_PATH}", server_url)
}

async fn index(user: Option<User>) -> Markup {
    page(
        "grand music cup",
        html! { "welcome to my page" },
        user.as_ref(),
    )
}

#[derive(Debug, Deserialize)]
struct DiscordAuthParams {
    code: String,
}

fn user_token_response(
    user: &User,
    jwt_secret: &[u8],
    next_location: &str,
) -> Result<Response, AppError> {
    let (token, max_age) = generate_user_token(user, jwt_secret)?;
    let cookie = Cookie::build(("token", token))
        .path("/")
        // TODO unify Duration across project
        .max_age(SignedDuration::milliseconds(max_age.as_millis() as i64))
        .build();
    let mut response = Redirect::to(next_location).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string())
            .expect("token header value should not be invalid"),
    );

    Ok(response)
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    login_name: String,
    password: String,
}

async fn post_log_in(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::authenticate(&form.login_name, &form.password, &state.config.pool)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    user_token_response(&user, &state.config.jwt_secret, "/")
}

async fn get_log_in(user: Option<User>) -> Result<impl IntoResponse, AppError> {
    user.is_none().ok_or(AppError::LoggedIn)?;

    Ok(html! {
        form method="POST" {
            label for="login name" { "login name:" }
            input type="text" name="login_name" required {}
            // label for="display name" { "your display name:" }
            // input type="text" name="display name" required {}
            label for="password" { "password:" }
            input type="password" name="password" required {}
            input type="submit" value="log in" {}
        }
    })
}

#[derive(Debug, Deserialize)]
struct SignUpForm {
    login_name: String,
    display_name: String,
    password: String,
}

async fn post_sign_up(
    State(state): State<AppState>,
    Form(form): Form<SignUpForm>,
) -> Result<impl IntoResponse, AppError> {
    let my_rng = state.rng.clone();
    let mut lock = my_rng.lock().await;
    let user = User::create_with_login_name(
        &state.config.snowflake_manager,
        &form.display_name,
        &form.login_name,
        &form.password,
        &mut lock,
        &state.config.pool,
    )
    .await?;

    user_token_response(&user, &state.config.jwt_secret, "/")
}

async fn get_sign_up(user: Option<User>) -> Result<impl IntoResponse, AppError> {
    user.is_none().ok_or(AppError::LoggedIn)?;

    Ok(html! {
        form method="POST" {
            label for="login name" { "login name:" }
            input type="text" name="login_name" required {}
            label for="display name" { "display name:" }
            input type="text" name="display_name" required {}
            label for="password" { "password:" }
            input type="password" name="password" required {}
            input type="submit" value="log in" {}
        }
    })
}

async fn discord_auth(
    State(state): State<AppState>,
    Query(params): Query<DiscordAuthParams>,
) -> Result<impl IntoResponse, AppError> {
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

    user_token_response(&user, &state.config.jwt_secret, "/")
}
