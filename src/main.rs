use axum::{Router, extract::State, http::Uri, routing::method_routing::get};
use clap::Parser;
use maud::{Markup, html};
use rspotify::{AuthCodeSpotify, Credentials, OAuth, clients::OAuthClient};
use urlencoding::encode;
use std::{error::Error, sync::Arc};
use url::{Url, form_urlencoded};

use crate::get_code::get_code;

mod get_code;

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
    #[arg(short = 'd', long, env = "DISCORD_CLIENT_ID")]
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
}

#[derive(Clone, Debug)]
struct AppState {
    discord_authorization_url: Arc<str>,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv()?;
    let args = Args::parse();

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

    let discord_authorization_url = Arc::from(build_discord_authorization_url(
        &args.discord_client_id,
        &["identify"],
        &format!("http://{}/discord/authorize", args.server_address),
        None,
    )?);

    let state = AppState {
        discord_authorization_url
    };

    let routes = Router::new().route("/", get(root)).with_state(state);
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

    Ok(Url::parse_with_params(
        &format!("{DISCORD_URL}/oauth2/authorize"),
        &params,
    )?.to_string())
}

async fn root(State(state): State<AppState>) -> Markup {
    html! {
        a href=(state.discord_authorization_url) { "discord auth" }
    }
}
