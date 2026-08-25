use axum::Router;
use base64::prelude::*;
use std::{error::Error, process::exit, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    config::Mode,
    crypto::{init_rng, random_bytes},
    web::{WebState, app},
};

mod auth;
mod config;
mod crypto;
mod database;
mod discord;
mod snowflake;
mod spotify;
mod web;

const SECRET_LEN: usize = 32;
pub const APP_PATH: &str = "";
pub const API_PATH: &str = "/api";

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (web_config, server_config, mut join_set) = match config::get_config().await? {
        Mode::Cli(_cli_config) => {
            println!(
                "{}",
                BASE64_STANDARD.encode(random_bytes::<SECRET_LEN>(&mut init_rng()))
            );
            exit(0);
        }
        Mode::WebServer(config) => config,
    };

    let routes = Router::new()
        .merge(app::routes())
        // .nest(API_PATH, api::routes())
        .with_state(Arc::new(WebState {
            config: web_config,
            rng: Mutex::new(init_rng()),
        }));

    let listener = tokio::net::TcpListener::bind(server_config.server_address.as_ref()).await?;
    axum::serve(listener, routes)
        .with_graceful_shutdown(async move {
            println!(
                "task finished unexpectedly: {:?}",
                join_set.join_next().await
            );
        })
        .await?;

    Ok(())
}

// #[derive(Debug, Serialize, Deserialize)]
// struct AuthTokenClaims {
//     exp: usize,
//     sub: i64,
// }
//
// async fn root(State(state): State<AppState>) -> Markup {
//     html! {
//         a href=(state.discord_authorization_url) { "discord auth" }
//     }
// }
//
// async fn discord_signup(
//     cookies: CookieJar,
//     State(state): State<AppState>,
//     req: Request,
// ) -> Result<Markup, (StatusCode, Markup)> {
//     let error_markup = html! {
//         a href=(state.discord_authorization_url) { "try again" }
//     };
//
//     println!("{:?}", req.headers());
//     let hi = decode::<DiscordSignUpTokenClaims>(
//         cookies
//             .get("token")
//             .map(|cookie| cookie.value().to_string())
//             .ok_or((
//                 StatusCode::UNAUTHORIZED,
//                 html! {
//                     p {
//                         "no cookie."
//                         a href=(state.discord_authorization_url) { "try again" }
//                     }
//                 },
//             ))?,
//         &DecodingKey::from_secret(&state.jwt_secret),
//         &Validation::default(),
//     )
//     .inspect_err(|e| println!("{e}"))
//     .map_err(|_| {
//         (
//             StatusCode::UNAUTHORIZED,
//             html! {
//                     p {
//                         "could not decode token."
//                         a href=(state.discord_authorization_url) { "try again" }
//                     }
//             },
//         )
//     })?;
//     Ok(format!("{:?}", hi.claims).render())
// }
