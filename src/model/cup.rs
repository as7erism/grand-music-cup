use std::num::NonZeroU32;

use rspotify::model::TrackId;
use serde::Deserialize;

use crate::model::user::{User, UserId};

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
}

#[derive(Debug)]
pub struct Vote<'u> {
    user_id: UserId<'u>,
    count: u32,
}

#[derive(Debug)]
pub enum RoundPhase<'u, 't> {
    NotStarted,
    // TODO vec to mark mutability is interesting but probably doesn't matter -
    // these can probably just be Boxes
    Submission(Vec<TrackSubmission<'u, 't>>),
    Voting(Vec<(TrackSubmission<'u, 't>, Vec<Vote<'u>>)>),
    Finished(Box<[(TrackSubmission<'u, 't>, Box<[Vote<'u>]>)]>),
}

#[derive(Debug)]
pub struct Round<'u, 't> {
    name: String,
    description: String,
    phase: RoundPhase<'u, 't>,
}

#[derive(Debug)]
pub struct Cup<'u, 't> {
    id: i64,
    name: String,
    owner: UserId<'u>,
    current_round: Option<usize>,
    rounds: Vec<Round<'u, 't>>,
    timestamp_ms: u64,
    players: Vec<UserId<'u>>,
}
