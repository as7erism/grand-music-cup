use std::{collections::HashSet, convert::Infallible, sync::Arc, time::Duration};

use rspotify::{
    AuthCodeSpotify, ClientError, Credentials, OAuth,
    clients::{BaseClient, OAuthClient},
    model::IdError,
};
use thiserror::Error;
use tokio::{task::JoinHandle, time::interval};

const SCOPES: [&str; 1] = ["playlist-modify-public"];

#[derive(Debug, Error)]
pub enum SpotifyError {
    #[error("client error: {0}")]
    ClientError(#[from] ClientError),
    #[error("id error: {0}")]
    IdError(#[from] IdError),
}

pub struct SpotifyClient {
    inner: Arc<AuthCodeSpotify>,
}

impl SpotifyClient {
    pub async fn new(
        config: SpotifyClientConfig,
    ) -> Result<
        (
            Self,
            JoinHandle<Result<Infallible, Box<dyn std::error::Error + Send + Sync>>>,
        ),
        SpotifyError,
    > {
        let oauth = OAuth {
            redirect_uri: config.oauth_callback_url,
            scopes: SCOPES
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
            ..Default::default()
        };
        let credentials = Credentials::new(&config.client_id, &config.client_secret);
        let client = Arc::new(AuthCodeSpotify::new(credentials, oauth));

        let code = match config.authorization_code {
            Some(code) => code,
            None => client.get_code_from_user(&client.get_authorize_url(true)?)?,
        };
        client.request_token(&code).await?;

        let client_clone = client.clone();
        let wrapped = Self { inner: client };

        let handle = tokio::spawn(async move {
            client_clone.refresh_token().await?;

            let mut interval = interval(config.refresh_interval);
            loop {
                interval.tick().await;
                client_clone.refresh_token().await?;
            }
        });

        Ok((wrapped, handle))
    }
}

pub struct SpotifyClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub oauth_callback_url: String,
    pub authorization_code: Option<String>,
    pub refresh_interval: Duration,
}
