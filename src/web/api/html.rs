use axum::{Router, extract::Query, routing::get};
use maud::{Markup, html};
use serde::Deserialize;

use crate::web::app::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/player-cap-checkbox", get(player_cap_checkbox))
}

#[derive(Debug, Deserialize)]
struct PlayerCapToggle {
    has_player_cap: Option<String>,
}

async fn player_cap_checkbox(Query(params): Query<PlayerCapToggle>) -> Markup {
    html! {
        @if let Some(cap) = params.has_player_cap && cap == "on" {
            label for="max_players" .text-sm { "max players: " }
            br;
            input type="number" min="2" max="200" name="max_players" .border.p-1 {}
            br;
        }
    }
}
