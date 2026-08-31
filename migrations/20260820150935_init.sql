-- Add migration script here
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY NOT NULL,
  display_name TEXT NOT NULL,

  discord_id TEXT UNIQUE,

  login_name TEXT UNIQUE,
  salt BLOB,
  password_hash BLOB
);

CREATE TABLE IF NOT EXISTS cups
(
  id INTEGER PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  owner_id INTEGER NOT NULL,
  max_players INTEGER CHECK(max_players > 1 AND max_players < 200),

  vote_allocation INTEGER NOT NULL CHECK(vote_allocation > 0 AND vote_allocation < 100),
  submission_time_ms INTEGER NOT NULL CHECK(submission_time_ms > 0 AND submission_time_ms < 1209600000), -- two weeks
  voting_time_ms INTEGER NOT NULL CHECK(voting_time_ms > 0 AND voting_time_ms < 1209600000), -- two weeks

  current_round_number INTEGER,
  next_action_timestamp_ms INTEGER,

  FOREIGN KEY(owner_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS rounds
(
  round_number INTEGER NOT NULL,
  cup_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  phase TEXT CHECK(phase in ('NotStarted', 'Submission', 'Voting', 'Finished')) NOT NULL DEFAULT 'NotStarted',

  FOREIGN KEY(cup_id) REFERENCES cups(id),
  UNIQUE(round_number, cup_id),
  UNIQUE(name, cup_id)
);

CREATE TABLE IF NOT EXISTS round_submissions
(
  round_number INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  cup_id INTEGER NOT NULL,
  track_id TEXT NOT NULL,
  pre_voting_note TEXT NOT NULL DEFAULT '',
  post_voting_note TEXT NOT NULL DEFAULT '',

  FOREIGN KEY(round_number, cup_id) REFERENCES rounds(round_number, cup_id),
  FOREIGN KEY(user_id, cup_id) REFERENCES cup_participants(user_id, cup_id),
  UNIQUE(round_number, cup_id, user_id)
);

CREATE TABLE IF NOT EXISTS votes
(
  round_number INTEGER NOT NULL,
  cup_id INTEGER NOT NULL,
  submitter_id INTEGER NOT NULL,
  voter_id INTEGER NOT NULL,
  count INTEGER NOT NULL,
  note TEXT,

  FOREIGN KEY(round_number, cup_id, submitter_id) REFERENCES round_submissions(round_number, cup_id, user_id),
  FOREIGN KEY(voter_id) REFERENCES users(id),
  CHECK(submitter_id != voter_id),
  UNIQUE(round_number, cup_id, submitter_id, voter_id)
);

CREATE TABLE IF NOT EXISTS cup_participants
(
  user_id INTEGER NOT NULL,
  cup_id INTEGER NOT NULL,
  is_active INTEGER NOT NULL DEFAULT TRUE,

  FOREIGN KEY(user_id) REFERENCES users(id),
  FOREIGN KEY(cup_id) REFERENCES cups(id),
  UNIQUE(user_id, cup_id)
);
