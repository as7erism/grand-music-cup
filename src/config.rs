use std::{convert::Infallible, error::Error, time::Duration};

use base64::prelude::*;
use clap::{Args, Parser};
use grand_music_cup::U10;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tokio::task::{JoinError, JoinSet};

use crate::{
    discord::DiscordClient,
    spotify::{SpotifyClient, SpotifyClientConfig},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Config {
    /// The Spotify client ID
    #[arg(short = 'i', long, env = "SPOTIFY_CLIENT_ID")]
    spotify_client_id: String,

    /// The Spotify client secret
    #[arg(short = 's', long, env = "SPOTIFY_CLIENT_SECRET")]
    spotify_client_secret: String,

    /// The Discord client ID
    #[arg(short = 'r', long, env = "DISCORD_CLIENT_ID")]
    discord_client_id: String,

    /// The Discord client secret
    #[arg(short = 'e', long, env = "DISCORD_CLIENT_SECRET")]
    discord_client_secret: String,

    /// The OAuth callback URL (must match Spotify App configuration)
    #[arg(
        short = 'o',
        long,
        env = "OAUTH_CALLBACK_URL",
        default_value = "http://127.0.0.1:8463"
    )]
    spotify_oauth_callback_url: String,

    /// The code used to get an access token for the Spotify client
    #[arg(short = 'z', long, env = "SPOTIFY_AUTHORIZATION_CODE")]
    spotify_authorization_code: Option<String>,

    /// The interval at which the Spotify client token should be refreshed
    #[arg(
        short = 'f',
        long,
        env = "SPOTIFY_REFRESH_INTERVAL_MINUTES",
        default_value_t = 30
    )]
    spotify_refresh_interval_minutes: u64,

    /// The live server address
    #[arg(
        short = 'a',
        long,
        env = "SERVER_ADDRESS",
        default_value = "127.0.0.1:8464"
    )]
    server_address: String,

    #[command(flatten)]
    https_config: Option<HttpsConfig>,

    /// The database connection string
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// The JWT secret seed, base64-encoded
    #[arg(short, long, env = "JWT_SECRET")]
    jwt_secret: Option<String>,

    /// The epoch as milliseconds since the unix epoch
    #[arg(short = 'p', long, env = "EPOCH_MS", default_value_t = 1787681355986)]
    epoch_ms: u64,

    /// The machine ID (should fit within 10 bits)
    #[arg(short, long, env = "MACHINE_ID", default_value_t = 0)]
    machine_id: u16,

    /// If enabled, program will generate a suitable JWT secret seed and exit
    #[arg(short, long)]
    generate_secret: bool,
}

#[derive(Args, Debug)]
#[group(required = false)]
pub struct HttpsConfig {
    #[arg(short = 't', long, required = false)]
    pub cert_file: String,

    #[arg(short, long, required = false)]
    pub key_file: String,
}

#[derive(Debug)]
pub struct WebConfig {
    pub discord_client: DiscordClient,
    pub spotify_client: SpotifyClient,
    pub pool: SqlitePool,
    pub machine_id: U10,
    pub epoch_ms: u64,
    pub jwt_secret: Box<[u8]>,
    pub server_url: Box<str>,
}

pub struct ServerConfig {
    pub server_address: Box<str>,
    pub https_config: Option<HttpsConfig>,
}

pub struct CliConfig {}

pub type TaskJoinSet = JoinSet<Result<Result<Infallible, Box<dyn Error + Send + Sync>>, JoinError>>;

pub enum Mode {
    Cli(CliConfig),
    WebServer((WebConfig, ServerConfig, TaskJoinSet)),
}

pub async fn get_config() -> Result<Mode, Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv()?;
    let config = Config::parse();

    if config.generate_secret {
        return Ok(Mode::Cli(CliConfig {}));
    }

    let discord_client = DiscordClient {
        client_id: config.discord_client_id,
        client_secret: config.discord_client_secret,
    };

    let spotify_client_config = SpotifyClientConfig {
        client_id: config.spotify_client_id,
        client_secret: config.spotify_client_secret,
        oauth_callback_url: config.spotify_oauth_callback_url,
        authorization_code: config.spotify_authorization_code,
        refresh_interval: Duration::from_mins(config.spotify_refresh_interval_minutes),
    };
    let (spotify_client, spotify_client_handle) = SpotifyClient::new(spotify_client_config).await?;

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    let machine_id =
        U10::new(config.machine_id).expect("machine id overflow; TODO this should be a result");

    let jwt_secret = BASE64_STANDARD
        .decode(
            config
                .jwt_secret
                .expect("secret missing; TODO this should be mapped to a result"),
        )?
        .into_boxed_slice();

    let server_url = if config.https_config.is_some() {
        format! {"https://{}", config.server_address}.into_boxed_str()
    } else {
        format! {"http://{}", config.server_address}.into_boxed_str()
    };

    let web_config = WebConfig {
        spotify_client,
        discord_client,
        pool,
        epoch_ms: config.epoch_ms,
        machine_id,
        jwt_secret,
        server_url,
    };

    let server_config = ServerConfig {
        server_address: config.server_address.into_boxed_str(),
        https_config: config.https_config,
    };

    let join_set: TaskJoinSet = [Box::new(spotify_client_handle)].into_iter().collect();

    Ok(Mode::WebServer((web_config, server_config, join_set)))
}
