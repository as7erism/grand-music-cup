use clap::Parser;
use rspotify::{AuthCodeSpotify, Credentials, OAuth, clients::OAuthClient};
use std::{error::Error, sync::Arc};

use crate::get_code::get_code;

mod get_code;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The client ID
    #[arg(short = 'c', long, env = "RSPOTIFY_CLIENT_ID")]
    client_id: String,

    /// The client secret
    #[arg(short = 's', long, env = "RSPOTIFY_CLIENT_SECRET")]
    client_secret: String,

    /// The OAuth callback URL (must match Spotify App configuration)
    #[arg(
        short = 'u',
        long,
        env = "OAUTH_CALLBACK_URL",
        default_value = "127.0.0.1"
    )]
    callback_url: String,

    /// The OAuth callback port (must match Spotify App configuration)
    #[arg(short = 'p', long, env = "OAUTH_CALLBACK_PORT", default_value_t = 8463)]
    callback_port: u16,

    /// The code used to get an access token for the Spotify client
    #[arg(short, long, env = "AUTHORIZATION_CODE")]
    authorization_code: Option<String>,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv()?;
    let args = Args::parse();

    let oauth = OAuth {
        redirect_uri: format!("http://{}:{}", args.callback_url, args.callback_port),
        scopes: ["playlist-modify-public".into()].into(),
        ..Default::default()
    };
    let credentials = Credentials::new(&args.client_id, &args.client_secret);
    let client = Arc::new(AuthCodeSpotify::new(credentials, oauth));

    let code = match args.authorization_code {
        Some(code) => code,
        None => client.get_code_from_user(&client.get_authorize_url(true)?)?,
    };
    println!("{code}");
    client.request_token(&code).await?;

    Ok(())
}
