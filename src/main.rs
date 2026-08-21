use axum::{Router, extract::State, routing::method_routing::get};
use base64::prelude::*;
use clap::Parser;
use maud::{Markup, html};
use rand::{
    SeedableRng, TryRng,
    rngs::{StdRng, SysRng},
};
use rspotify::{AuthCodeSpotify, Credentials, OAuth, clients::OAuthClient};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::{error::Error, sync::Arc};
use url::Url;

use crate::api::{ApiState, init_api};

mod api;

const DISCORD_URL: &str = "https://discord.com";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
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

    /// The OAuth callback address (must match Spotify App configuration)
    #[arg(
        short = 'o',
        long,
        env = "OAUTH_CALLBACK_ADDRESS",
        default_value = "127.0.0.1:8463"
    )]
    oauth_callback_address: String,

    /// The code used to get an access token for the Spotify client
    #[arg(short = 'u', long, env = "AUTHORIZATION_CODE")]
    authorization_code: Option<String>,

    /// The live server address
    #[arg(
        short = 'a',
        long,
        env = "SERVER_ADDRESS",
        default_value = "127.0.0.1:8464"
    )]
    server_address: String,

    /// The database connection string
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// The JWT secret seed, base64-encoded
    #[arg(short, long, env = "JWT_SECRET")]
    jwt_secret: Option<String>,

    /// If enabled, program will generate a suitable JWT secret seed and exit
    #[arg(short, long)]
    generate_secret: bool,
}

#[derive(Clone, Debug)]
struct AppState {
    pool: SqlitePool,
    discord_authorization_url: Arc<str>,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv()?;
    let args = Args::parse();

    if args.generate_secret {
        let mut buffer: [u8; 32] = [0; 32];
        let mut rng = StdRng::try_from_rng(&mut SysRng)?;
        rng.try_fill_bytes(&mut buffer)?;
        println!("{}", BASE64_STANDARD.encode(buffer));
        return Ok(());
    }

    let Some(jwt_secret) = args.jwt_secret else {
        panic!("jwt secret is required; generate one with --generate-secret");
    };
    let jwt_secret = BASE64_STANDARD.decode(jwt_secret)?;

    let oauth = OAuth {
        redirect_uri: format!("http://{}", args.oauth_callback_address),
        scopes: ["playlist-modify-public".into()].into(),
        ..Default::default()
    };
    let credentials = Credentials::new(&args.spotify_client_id, &args.spotify_client_secret);
    let client = Arc::new(AuthCodeSpotify::new(credentials, oauth));

    let code = match args.authorization_code {
        Some(code) => code,
        None => client.get_code_from_user(&client.get_authorize_url(true)?)?,
    };
    println!("{code}");
    client.request_token(&code).await?;

    let discord_authorization_callback_url = Arc::from(format!(
        "http://{}/api/html/authorize/discord",
        args.server_address
    ));
    let discord_authorization_url = Arc::from(build_discord_authorization_url(
        &args.discord_client_id,
        &["identify"],
        &discord_authorization_callback_url,
        None,
    )?);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&args.database_url)
        .await?;

    let state = AppState {
        pool: pool.clone(),
        discord_authorization_url,
    };

    let routes = Router::new()
        .route("/", get(root))
        .with_state(state)
        .nest("/api", init_api())
        .with_state(ApiState {
            discord_authorization_callback_url,
            discord_client_id: Arc::from(args.discord_client_id),
            discord_client_secret: Arc::from(args.discord_client_secret),
            server_address: Arc::from(args.server_address.clone()),
            jwt_secret: Arc::from(jwt_secret),
            pool,
        });
    let listener = tokio::net::TcpListener::bind(args.server_address).await?;
    axum::serve(listener, routes).await?;

    Ok(())
}

fn build_discord_authorization_url(
    client_id: &str,
    scopes: &[&str],
    callback_url: &str,
    state: Option<&str>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let scopes = scopes.join(" ");
    let mut params = vec![
        ("response_type", "code"),
        ("client_id", client_id),
        ("scope", &scopes),
        ("redirect_uri", callback_url),
    ];

    if let Some(state) = state {
        params.push(("state", state));
    }

    Ok(Url::parse_with_params(&format!("{DISCORD_URL}/oauth2/authorize"), &params)?.to_string())
}

async fn root(State(state): State<AppState>) -> Markup {
    html! {
        a href=(state.discord_authorization_url) { "discord auth" }
    }
}
