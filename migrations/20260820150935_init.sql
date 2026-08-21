-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY NOT NULL,
  username TEXT UNIQUE NOT NULL,

  discord_id TEXT UNIQUE,
  discord_username TEXT,
  discord_avatar_hash TEXT,

  salt TEXT,
  password_hash TEXT
);
