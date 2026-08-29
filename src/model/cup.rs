use std::num::NonZeroU32;

use rspotify::model::TrackId;
use serde::Deserialize;
use sqlx::SqlitePool;
use time::{Duration, SignedDuration};

use crate::model::{
    ModelError,
    user::{User, UserId},
};

#[derive(Debug, Deserialize)]
struct CupCreateParams {
    pub cup_name: String,
    pub submission_ms: u64,
    pub voting_ms: u64,
    pub vote_allocation: u32,
    pub max_players: Option<NonZeroU32>,
}

#[derive(Debug, Deserialize)]
pub struct RoundCreateParams {
    pub round_name: String,
    pub round_description: String,
}

#[derive(Debug)]
pub struct TrackSubmission<'u, 't> {
    user_id: UserId<'u>,
    track_id: TrackId<'t>,
    pre_voting_note: Option<String>,
    post_voting_note: Option<String>,
}

#[derive(Debug)]
pub struct Vote<'u> {
    user_id: UserId<'u>,
    count: u32,
}

#[derive(Debug)]
pub enum RoundPhase {
    NotStarted,
    Submission,
    Voting,
    Finished,
}

#[derive(Debug)]
pub struct Round {
    name: String,
    description: String,
    phase: RoundPhase,
}

#[derive(Debug)]
pub struct Cup {
    id: i64,
    name: String,
    description: Option<String>,
    creation_timestamp_ms: u64,
    owner: UserId<'static>,
    max_players: Option<usize>,
    current_round_number: Option<usize>,
    submission_time: SignedDuration,
    voting_time: SignedDuration,
    next_action_timestamp_ms: Option<u64>,
}

impl Cup {
    pub async fn fetch(id: i64, pool: &SqlitePool) -> Result<Self, ModelError> {
        let response = sqlx::query!(
            "
        SELECT *
        FROM cups
        WHERE id = ?
        ",
            id
        )
        .fetch_one(pool)
        .await?;

        Ok(Self {
            id: response.id,
            name: response.name,
            description: response.description,
            creation_timestamp_ms: response.creation_timestamp_ms as u64,
            owner: UserId::PrimaryKey(response.owner_id),
            max_players: response.max_players.map(|i| i as usize),
            current_round_number: response.current_round_number.map(|i| i as usize),
            submission_time: SignedDuration::milliseconds(response.submission_time_ms),
            voting_time: SignedDuration::milliseconds(response.voting_time_ms),
            next_action_timestamp_ms: response.next_action_timestamp_ms.map(|t| t as u64),
        })
    }

    pub async fn participant_ids(
        &self,
        pool: &SqlitePool,
    ) -> Result<Vec<(UserId<'static>, bool)>, ModelError> {
        Ok(sqlx::query!(
            "
        SELECT user_id, did_leave
        FROM cup_participants
        WHERE cup_id = ?
        ",
            self.id
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|response| {
            (
                UserId::PrimaryKey(response.user_id),
                response.did_leave != 0,
            )
        })
        .collect())
    }

    pub async fn participants(&self, pool: &SqlitePool) -> Result<Vec<(User, bool)>, ModelError> {
        Ok(sqlx::query!(
            "
        SELECT user_id, display_name, discord_id, login_name, did_leave
        FROM cup_participants
        JOIN users ON
            cup_participants.user_id = users.id
        WHERE cup_id = ?
        ",
            self.id
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|response| {
            (
                {
                    User {
                        id: response.user_id,
                        display_name: response.display_name,
                        discord_id: response.discord_id,
                        login_name: response.login_name,
                    }
                },
                response.did_leave != 0,
            )
        })
        .collect())
    }
}
