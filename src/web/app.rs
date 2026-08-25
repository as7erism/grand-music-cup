use std::sync::Arc;

use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use http::StatusCode;
use maud::{Markup, html};
use rand::rngs::StdRng;
use rspotify::sync::Mutex;
use serde::Deserialize;
use sqlx::SqlitePool;
use thiserror::Error;

use crate::{
    APP_PATH,
    database::{DatabaseError, User, UserId},
    discord::{DiscordError, DiscordUser},
    snowflake::SnowflakeManager,
    web::WebState,
};

const DISCORD_AUTH_PATH: &str = "/discord-auth";

#[derive(Debug, Error)]
pub enum AppError {
    // TODO we need to be careful not to be so leaky with error messages
    #[error("discord error: {0}")]
    DiscordError(#[from] DiscordError),
    #[error("database error: {0}")]
    DatabaseError(#[from] DatabaseError),
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
) -> Result<Markup, AppError> {
    let token = state
        .config
        .discord_client
        .exchange_code_for_token(&params.code, &discord_auth_url(&state.config.server_url))
        .await?;

    let discord_user = DiscordUser::get(&token).await?;
    if let Some(_existing_user) =
        User::fetch(UserId::DiscordId(&discord_user.id), &state.config.pool).await?
    {
        unimplemented!();
    }

    let user = User::create_with_discord_id(
        &state.config.snowflake_manager,
        &discord_user.username,
        &discord_user.id,
        &state.config.pool,
    )
    .await?;

    Ok(html! {
        (&format!("{:?}", user))
    })
}
