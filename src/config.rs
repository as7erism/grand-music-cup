use std::{convert::Infallible, error::Error, time::Duration};

use base64::prelude::*;
use clap::{Args, Parser};
use grand_music_cup::U10;
use rspotify::AuthCodeSpotify;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use crate::{
    discord::DiscordClient,
    snowflake::SnowflakeManager,
    spotify::{SpotifyClient, SpotifyClientConfig},
    web,
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
        default_value = 30
    )]
    spotify_refresh_interval_minutes: u64,

    /// The live server address
    #[arg(
        short = 'a',
        long,
        env = "SERVER_ADDRESS",
        default_value = "127.0.0.1:8464"
    )]
    server_adddress: String,

    #[command(flatten)]
    https_config: Option<HttpsConfig>,

    /// The database connection string
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// The JWT secret seed, base64-encoded
    #[arg(short, long, env = "JWT_SECRET")]
    jwt_secret: Option<String>,

    /// The epoch as milliseconds since the unix epoch
    #[arg(short, long, env = "EPOCH_MS", default_value = 1787681355986)]
    epoch_ms: u64,

    /// The machine ID (should fit within 10 bits)
    #[arg(short, long, env = "MACHINE_ID", default_value = 0)]
    machine_id: u16,

    /// If enabled, program will generate a suitable JWT secret seed and exit
    #[arg(short, long)]
    generate_secret: bool,
}

#[derive(Args)]
#[group(required = true)]
struct HttpsConfig {
    #[arg(short = 't', long, required = true)]
    cert_file: String,

    #[arg(short, long, required = true)]
    key_file: String,
}

pub struct WebConfig {
    pub discord_client: DiscordClient,
    pub spotify_client: SpotifyClient,
    pub pool: SqlitePool,
    pub snowflake_manager: SnowflakeManager,
    pub jwt_secret: Box<[u8]>,
    pub server_url: Box<str>,
}

struct ServerConfig {
    pub https_config: Option<HttpsConfig>,
}

pub struct CliConfig {}

pub enum Mode {
    Cli(CliConfig),
    WebServer(
        (
            WebConfig,
            ServerConfig,
            Vec<Box<dyn Future<Output = Result<Infallible, Box<dyn Error + Send + Sync>>>>>,
        ),
    ),
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
        client_secret: config.discord_client_secret,
        oauth_callback_url: config.spotify_oauth_callback_url,
        authorization_code: config.spotify_authorization_code,
        refresh_interval: Duration::from_mins(config.spotify_refresh_interval_minutes),
    };
    let (spotify_client, spotify_client_handle) = SpotifyClient::new(spotify_client_config).await?;

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    let snowflake_manager = SnowflakeManager::new(config.epoch_ms, U10::new(config.machine_id)?)?;

    let jwt_secret = BASE64_STANDARD
        .decode(config.jwt_secret?)?
        .into_boxed_slice();

    let server_url = if config.https_config.is_some() {
        format! {"https://{}", config.server_adddress}.into_boxed_str()
    } else {
        format! {"http://{}", config.server_adddress}.into_boxed_str()
    };

    let web_config = WebConfig {
        spotify_client,
        discord_client,
        pool,
        snowflake_manager,
        jwt_secret,
        server_url,
    };

    let server_config = ServerConfig {
        https_config: config.https_config,
    };

    let join_handles: Vec<
        Box<dyn Future<Output = Result<Infallible, Box<dyn Error + Send + Sync>>>>,
    > = vec![Box::new(spotify_client_handle)];

    Ok(Mode::WebServer((web_config, server_config, join_handles)))
}
