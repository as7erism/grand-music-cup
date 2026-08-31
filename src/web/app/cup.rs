use axum::{
    Form, Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use maud::html;

use crate::{
    model::{
        cup::{Cup, CupCreateParams},
        user::{LoginParams, User, UserId},
    },
    web::app::{
        AppError, AppState,
        views::{reactive_page, static_page},
    },
};

pub const CUP_CREATE_PATH: &str = "/create";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(CUP_CREATE_PATH, get(get_create))
        .route(CUP_CREATE_PATH, post(post_create))
}

async fn get_create(user: User, State(state): State<AppState>) -> impl IntoResponse {
    reactive_page(
        "create cup",
        html! {
            div .flex.flex-col.items-center.pt-8 {
                div {
                    form method="POST" {
                        label for="cup_name" .text-sm { "cup name:" }
                        br;
                        input type="text" name="cup_name" .border.p-1 required {}
                        br;

                        label for="cup_description" .text-sm { "description:" }
                        br;
                        input type="text" name="cup_description" .border.p-1 required {}
                        br;

                        label for="submission_time_ms" .text-sm { "submission time in milliseconds:" }
                        br;
                        input type="number" min="0" max="1209600000" name="submission_time_ms" .border.p-1 required {}
                        br;

                        label for="voting_time_ms" .text-sm { "voting time in milliseconds:" }
                        br;
                        input type="number" min="0" max="1209600000" name="voting_time_ms" .border.p-1 required {}
                        br;

                        label for="vote_allocation" .text-sm { "vote allocation:" }
                        br;
                        input type="number" min="1" max="100" name="vote_allocation" .border.p-1 required {}
                        br;

                        label for="has_player_cap" .text-sm { "cap players?" }
                        br;
                        input
                            type="checkbox"
                            hx-trigger="click"
                            hx-get="/api/html/player-cap-checkbox"
                            hx-target="#player-cap-container"
                            name="has_player_cap"
                        {}
                        br;
                        div id="player-cap-container" {
                        }
                        div .flex.justify-center.pt-2 {
                            input type="submit" .text-md.cursor-pointer.text-mauve-700.hover:text-mauve-500 value="create" {}
                        }
                    }
                }
            }
        },
        Some(&user),
    )
}

async fn post_create(
    user: User,
    State(state): State<AppState>,
    Form(form): Form<CupCreateParams>,
) -> Result<impl IntoResponse, AppError> {
    let _cup = Cup::create(
        UserId::PrimaryKey(user.id()),
        &form,
        state.config.epoch,
        state.config.machine_id,
        &state.config.pool,
    )
    .await?;
    Ok(())
}
