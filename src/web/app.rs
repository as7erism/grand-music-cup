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
    discord::{DiscordError, DiscordUser},
    model::{
        ModelError,
        user::{User, UserId},
    },
    token::{TokenError, authenticate_user_token, generate_user_token},
    web::app::{AppError::LoggedIn, views::page},
};

mod auth;
mod cup;
mod views;

const AUTH_PATH: &str = "";

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
    DatabaseError(#[from] ModelError),
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
    Router::new().route("/", get(index)).merge(auth::routes())
}

async fn index(user: Option<User>) -> Markup {
    page(
        "grand music cup",
        html! {
            div .flex.justify-center.pt-12 {
                @if user.is_some() {
                    div .border.p-2 {
                        a href="create-cup" { "create a cup" }
                    }
                } @else {
                    "log in to create a cup"
                }
            }
        },
        user.as_ref(),
    )
}
