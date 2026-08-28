use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};

use crate::web::app::AppState;

pub const CUP_CREATE_PATH: &str = "/create";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(CUP_CREATE_PATH, get(get_create))
        .route(CUP_CREATE_PATH, post(post_create))
}

async fn get_create() -> impl IntoResponse {}

async fn post_create() -> impl IntoResponse {}
