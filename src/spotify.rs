use std::collections::HashSet;

use rspotify::{AuthCodeSpotify, ClientError, Credentials, OAuth, clients::OAuthClient};

pub const SCOPES: HashSet<String> = HashSet::from(String::from("playlist-modify-public"));

pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub oauth_callback_url: String,
    pub authorization_code: Option<String>,
}

impl ClientConfig {
    pub async fn into_client(self) -> Result<AuthCodeSpotify, ClientError> {
        let oauth = OAuth {
            redirect_uri: format!("http://{}", self.oauth_callback_url),
            scopes: ["playlist-modify-public".into()].into(),
            ..Default::default()
        };
        let credentials = Credentials::new(&self.client_id, &self.client_secret);
        let client = AuthCodeSpotify::new(credentials, oauth);

        let code = match self.authorization_code {
            Some(code) => code,
            None => client.get_code_from_user(&client.get_authorize_url(true)?)?,
        };
        client.request_token(&code).await?;

        Ok(client)
    }
}
