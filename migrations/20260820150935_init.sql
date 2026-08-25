-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY NOT NULL,
  display_name TEXT NOT NULL,

  discord_id TEXT UNIQUE,

  login_name TEXT UNIQUE,
  salt TEXT,
  password_hash TEXT
);
