use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_extra::extract::cookie::Cookie;
use http::HeaderValue;
use http::header::SET_COOKIE;
use maud::html;
use serde::Deserialize;
use time::SignedDuration;

use crate::APP_PATH;
use crate::discord::DiscordUser;
use crate::model::user::{LoginParams, SignUpParams, User, UserId};
use crate::token::generate_user_token;
use crate::web::app::views::static_page;
use crate::web::app::{AUTH_PATH, AppError, AppState};

pub const LOG_IN_PATH: &str = "/log-in";
pub const DISCORD_AUTH_PATH: &str = "/log-in/discord";
pub const SIGN_UP_PATH: &str = "/sign-up";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(LOG_IN_PATH, get(get_log_in))
        .route(LOG_IN_PATH, post(post_log_in))
        .route(DISCORD_AUTH_PATH, get(discord_auth))
        .route(SIGN_UP_PATH, get(get_sign_up))
        .route(SIGN_UP_PATH, post(post_sign_up))
}

fn discord_auth_url(server_url: &str) -> String {
    format!("{}{APP_PATH}{AUTH_PATH}{DISCORD_AUTH_PATH}", server_url)
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

async fn post_log_in(
    State(state): State<AppState>,
    Form(form): Form<LoginParams>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::authenticate(&form, &state.config.pool)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    user_token_response(&user, &state.config.jwt_secret, "/")
}

async fn get_log_in(
    user: Option<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    user.is_none().ok_or(AppError::LoggedIn)?;

    Ok(static_page(
        "log in",
        html! {
            div .flex.flex-col.items-center.pt-8 {
                div .text-md {
                    a href=(&state.config.discord_client.get_authorization_url(&discord_auth_url(&state.config.server_url), None)) .text-mauve-700.hover:text-mauve-500.cursor-pointer {
                        "log in with discord"
                    }
                    " or..."
                }
                div {
                    form method="POST" {
                        label for="login_name" .text-sm { "login name:" }
                        br;
                        input type="text" name="login_name" .border.p-1 required {}
                        br;
                        label for="password" .text-sm { "password:" }
                        br;
                        input type="password" name="password" .border.p-1 required {}
                        div .flex.justify-center.pt-2 {
                            input type="submit" .text-md.cursor-pointer.text-mauve-700.hover:text-mauve-500 value="log in" {}
                        }
                    }
                }
            }
        },
        user.as_ref(),
    ))
}

async fn post_sign_up(
    State(state): State<AppState>,
    Form(form): Form<SignUpParams>,
) -> Result<impl IntoResponse, AppError> {
    let my_rng = state.rng.clone();
    let mut lock = my_rng.lock().await;
    let user = User::create_with_login_name(
        &form,
        &mut lock,
        state.config.epoch,
        state.config.machine_id,
        &state.config.pool,
    )
    .await?;

    user_token_response(&user, &state.config.jwt_secret, "/")
}

async fn get_sign_up(
    user: Option<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    user.is_none().ok_or(AppError::LoggedIn)?;

    Ok(static_page(
        "sign up",
        html! {
            div .flex.flex-col.items-center.pt-8 {
                div .text-md {
                    a href=(
                        &state.config.discord_client.get_authorization_url(&discord_auth_url(&state.config.server_url), None)
                    ) .text-mauve-700.hover:text-mauve-500.cursor-pointer {
                        "sign up with discord"
                    }
                    " or..."
                }
                div {
                    form method="POST" {
                        label for="login_name" .text-sm { "login name:" }
                        br;
                        input type="text" name="login_name" .border.p-1 required {}
                        br;
                        label for="display_name" .text-sm { "display name:" }
                        br;
                        input type="text" name="display_name" .border.p-1 required {}
                        br;
                        label for="password" .text-sm { "password:" }
                        br;
                        input type="password" name="password" .border.p-1 required {}
                        div .flex.justify-center.pt-2 {
                            input type="submit" .text-md.cursor-pointer.text-mauve-700.hover:text-mauve-500 value="sign up" {}
                        }
                    }
                }
            }
        },
        user.as_ref(),
    ))
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
                &discord_user.username,
                &discord_user.id,
                state.config.epoch,
                state.config.machine_id,
                &state.config.pool,
            )
            .await?
        }
    };

    user_token_response(&user, &state.config.jwt_secret, "/")
}
