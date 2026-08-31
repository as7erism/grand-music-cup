use axum::Router;

use crate::web::app::AppState;

mod html;
mod json;
mod union;

pub fn routes() -> Router<AppState> {
    Router::new().nest("/html", html::routes())
}
