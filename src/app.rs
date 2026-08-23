use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AppState {
    pool: SqlitePool,
    jwt_secret: Arc<[u8]>,
    discord_authorization_url: Arc<str>,
}

#[derive(Clone, Debug, Error)]
pub enum AppError {}

pub fn routes() -> Router<AppState> {}
