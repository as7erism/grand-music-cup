use axum::Router;
use base64::prelude::*;
use std::{error::Error, process::exit, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    config::Mode,
    crypto::{init_rng, random_bytes},
    web::{
        app::{self, AppState},
        assets,
    },
};

mod config;
mod crypto;
mod discord;
mod model;
mod snowflake;
mod spotify;
mod token;
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

    let web_config = Arc::new(web_config);
    let rng = Arc::new(Mutex::new(init_rng()));
    let routes = Router::new()
        .merge(app::routes())
        // .nest(API_PATH, api::routes())
        .with_state(AppState {
            config: web_config.clone(),
            rng: rng.clone(),
        })
        .merge(assets::routes());

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
