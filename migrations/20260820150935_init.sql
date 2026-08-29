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
  description TEXT,
  creation_timestamp_ms INTEGER NOT NULL,
  owner_id INTEGER NOT NULL,
  max_players INTEGER CHECK(max_players > 0),

  current_round_number INTEGER,
  submission_time_ms INTEGER NOT NULL,
  voting_time_ms INTEGER NOT NULL,
  next_action_timestamp_ms INTEGER,

  FOREIGN KEY(owner_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS rounds
(
  round_number INTEGER NOT NULL,
  cup_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
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
  pre_voting_note TEXT,
  post_voting_note TEXT,

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
  did_leave INTEGER NOT NULL DEFAULT FALSE,

  FOREIGN KEY(user_id) REFERENCES users(id),
  FOREIGN KEY(cup_id) REFERENCES cups(id),
  UNIQUE(user_id, cup_id)
);
