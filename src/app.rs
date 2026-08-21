use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;

#[derive(Clone, Debug)]
pub struct AppState {
    pool: SqlitePool,
    jwt_secret: Arc<[u8]>,
    discord_authorization_url: Arc<str>,
}

pub fn routes() -> Router<AppState> {

}
