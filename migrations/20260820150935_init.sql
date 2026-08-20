-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY NOT NULL,
  username TEXT NOT NULL,

  discord_id TEXT,
  discord_refresh_token TEXT,
  discord_token_expiry_timestamp INTEGER,

  salt TEXT,
  password_hash TEXT,
);
