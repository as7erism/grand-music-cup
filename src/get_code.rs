use axum::debug_handler;
use axum::extract::{Query, State};
use axum::{Router, routing::get};
use rspotify::AuthCodeSpotify;
use rspotify::clients::OAuthClient;
use std::sync::Arc;
use tokio::net::ToSocketAddrs;

pub async fn get_code<A: ToSocketAddrs>(
    client: Arc<AuthCodeSpotify>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    client
        .prompt_for_token(&client.get_authorize_url(false)?)
        .await?;
    Ok(())
}
