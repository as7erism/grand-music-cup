use std::num::{NonZeroU32, NonZeroU64};

use grand_music_cup::U10;
use rspotify::model::TrackId;
use serde::Deserialize;
use sqlx::SqlitePool;
use time::{SignedDuration, UtcDateTime};

use crate::{
    model::{
        ModelError, i64_to_bool,
        user::{User, UserId},
    },
    snowflake::Snowflake,
};

#[derive(Debug)]
pub enum Participant<U> {
    Active(U),
    Inactive(U),
}

impl<U> Participant<U> {
    pub fn as_owned(self) -> U {
        match self {
            Self::Active(user) => user,
            Self::Inactive(user) => user,
        }
    }

    pub fn as_ref(&self) -> &U {
        match self {
            Self::Active(user) => user,
            Self::Inactive(user) => user,
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            Self::Active(_) => true,
            Self::Inactive(_) => false,
        }
    }

    pub fn is_inactive(&self) -> bool {
        !self.is_active()
    }
}

#[derive(Debug, Deserialize)]
pub struct CupCreateParams {
    pub cup_name: String,
    pub cup_description: String,
    pub submission_time_ms: i64,
    pub voting_time_ms: i64,
    pub vote_allocation: NonZeroU32,
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
    description: String,
    owner: UserId<'static>,
    max_players: Option<u32>,
    current_round_number: Option<usize>,
    submission_time: SignedDuration,
    voting_time: SignedDuration,
    next_action_timestamp: Option<UtcDateTime>,
    vote_allocation: u32,
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
            owner: UserId::PrimaryKey(response.owner_id),
            max_players: response.max_players.map(|m| m as u32),
            current_round_number: response.current_round_number.map(|i| i as usize),
            submission_time: SignedDuration::milliseconds(response.submission_time_ms),
            voting_time: SignedDuration::milliseconds(response.voting_time_ms),
            next_action_timestamp: response.next_action_timestamp_ms.map(|t| {
                UtcDateTime::from_unix_timestamp(t)
                    .expect("database should not have invalid timestamp")
            }),
            vote_allocation: response.vote_allocation as u32,
        })
    }

    pub async fn participant_ids(
        &self,
        pool: &SqlitePool,
    ) -> Result<Vec<Participant<UserId<'static>>>, ModelError> {
        Ok(sqlx::query!(
            "
        SELECT user_id, is_active
        FROM cup_participants
        WHERE cup_id = ?
        ",
            self.id
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|response| {
            if i64_to_bool(response.is_active) {
                Participant::Active(UserId::PrimaryKey(response.user_id))
            } else {
                Participant::Inactive(UserId::PrimaryKey(response.user_id))
            }
        })
        .collect())
    }

    pub async fn participants(
        &self,
        pool: &SqlitePool,
    ) -> Result<Vec<Participant<User>>, ModelError> {
        Ok(sqlx::query!(
            "
        SELECT user_id, display_name, discord_id, login_name, is_active
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
            let user = User {
                id: response.user_id,
                display_name: response.display_name,
                discord_id: response.discord_id,
                login_name: response.login_name,
            };
            if i64_to_bool(response.is_active) {
                Participant::Active(user)
            } else {
                Participant::Inactive(user)
            }
        })
        .collect())
    }

    pub async fn create(
        owner: UserId<'_>,
        params: &CupCreateParams,
        epoch: UtcDateTime,
        machine_id: U10,
        pool: &SqlitePool,
    ) -> Result<Self, ModelError> {
        let response = sqlx::query!(
            "
        INSERT INTO cups (id, name, description, owner_id, max_players, vote_allocation, submission_time_ms, voting_time_ms)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        ",
            Snowflake::new_unique(epoch, machine_id)?.as_i64(),
            params.cup_name,
            params.cup_description,
            owner.to_primary_key(pool).await?,
            params.max_players.map(|m| m.get()),
            params.vote_allocation.get(),
            params.submission_time_ms,
            params.voting_time_ms,
        )
        .fetch_one(pool)
        .await?;

        Ok(Self {
            id: response
                .id
                .expect("i literally don't know why this is an option"),
            name: response.name,
            description: response.description,
            owner: UserId::PrimaryKey(response.owner_id),
            max_players: response.max_players.map(|m| m as u32),
            current_round_number: response.current_round_number.map(|m| m as usize),
            submission_time: SignedDuration::milliseconds(response.submission_time_ms),
            voting_time: SignedDuration::milliseconds(response.voting_time_ms),
            next_action_timestamp: response.next_action_timestamp_ms.map(|t| {
                UtcDateTime::from_unix_timestamp(t)
                    .expect("database should not have invalid timestamp")
            }),
            vote_allocation: response.vote_allocation as u32,
        })
    }
}
