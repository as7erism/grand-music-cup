use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::method_routing::get,
};
use axum_extra::extract::CookieJar;
use base64::prelude::*;
use clap::{Args, Parser};
use grand_music_cup::generate_secret;
use jsonwebtoken::{DecodingKey, Validation, decode};
use maud::{Markup, Render, html};
use rand::{
    SeedableRng, TryRng,
    rngs::{StdRng, SysRng},
};
use rspotify::{AuthCodeSpotify, Credentials, OAuth, clients::OAuthClient};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::{error::Error, sync::Arc};
use url::Url;

use crate::{
    api::{ApiState, init_api},
    spotify::ClientConfig,
};

mod api;
mod app;
mod discord;
mod spotify;
mod user;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
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
    #[arg(short = 'u', long, env = "SPOTIFY_AUTHORIZATION_CODE")]
    spotify_authorization_code: Option<String>,

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

#[derive(Clone, Debug)]
struct AppState {
    pool: SqlitePool,
    jwt_secret: Arc<[u8]>,
    discord_authorization_url: Arc<str>,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv()?;
    let args = Cli::parse();

    if args.generate_secret {
        println!("{}", generate_secret());
        return Ok(());
    }

    let Some(jwt_secret) = args.jwt_secret.map(|secret| {
        Arc::from(
            BASE64_STANDARD
                .decode(secret)
                .expect("could not decode JWT secret as base64"),
        )
    }) else {
        panic!("jwt secret is required; generate one with --generate-secret");
    };

    let spotify_client = ClientConfig {
        client_id: args.spotify_client_id,
        client_secret: args.spotify_client_secret,
        oauth_callback_url: args.spotify_oauth_callback_url,
        authorization_code: args.spotify_authorization_code,
    }
    .into_client();

    let server_url: Arc<str> = Arc::from(format!(
        "{}://{}",
        if args.https_config.is_some() {
            "https"
        } else {
            "http"
        },
        args.server_adddress.clone()
    ));

    let discord_authorization_callback_url =
        Arc::from(format!("{}/api/html/authorize/discord", server_url.clone()));
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
        jwt_secret: jwt_secret.clone(),
        discord_authorization_url,
    };

    let routes = Router::new()
        .route("/", get(root))
        .route("/sign-up/discord", get(discord_signup))
        .with_state(state)
        .nest("/api", init_api())
        .with_state(ApiState {
            discord_authorization_callback_url,
            discord_client_id: Arc::from(args.discord_client_id),
            discord_client_secret: Arc::from(args.discord_client_secret),
            server_url,
            jwt_secret,
            pool,
        });
    // .route_layer(CorsLayer::new().allow_credentials(true).allow_origin("http://127.0.0.1:8464".parse::<HeaderValue>().unwrap()));
    let listener = tokio::net::TcpListener::bind(args.server_adddress).await?;
    axum::serve(listener, routes).await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthTokenClaims {
    exp: usize,
    sub: i64,
}

async fn root(State(state): State<AppState>) -> Markup {
    html! {
        a href=(state.discord_authorization_url) { "discord auth" }
    }
}

async fn discord_signup(
    cookies: CookieJar,
    State(state): State<AppState>,
    req: Request,
) -> Result<Markup, (StatusCode, Markup)> {
    let error_markup = html! {
        a href=(state.discord_authorization_url) { "try again" }
    };

    println!("{:?}", req.headers());
    let hi = decode::<DiscordSignUpTokenClaims>(
        cookies
            .get("token")
            .map(|cookie| cookie.value().to_string())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                html! {
                    p {
                        "no cookie."
                        a href=(state.discord_authorization_url) { "try again" }
                    }
                },
            ))?,
        &DecodingKey::from_secret(&state.jwt_secret),
        &Validation::default(),
    )
    .inspect_err(|e| println!("{e}"))
    .map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            html! {
                    p {
                        "could not decode token."
                        a href=(state.discord_authorization_url) { "try again" }
                    }
            },
        )
    })?;
    Ok(format!("{:?}", hi.claims).render())
}
