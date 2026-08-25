use std::sync::Arc;

use axum::Router;
use rand::rngs::StdRng;
use rspotify::sync::Mutex;
use sqlx::SqlitePool;
use thiserror::Error;

use crate::snowflake::SnowflakeManager;

#[derive(Clone, Debug)]
pub struct AppState {
    pool: SqlitePool,
    jwt_secret: Arc<[u8]>,
    snowflake_manager: &'static SnowflakeManager,
    rng: Arc<Mutex<StdRng>>,
}

#[derive(Clone, Debug, Error)]
pub enum AppError {}

pub fn routes() -> Router<AppState> {}
