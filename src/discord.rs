use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;
use url::Url;

pub const SCOPES: [&str; 1] = ["identify"];
pub const DISCORD_URL: &str = "https://discord.com";

pub struct DiscordClient {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error("Discord API Error: {0}")]
    ApiError(#[from] reqwest::Error),
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub avatar: String,
}

impl DiscordUser {
    pub async fn get(token: &str) -> Result<Self, DiscordError> {
        Ok(reqwest::Client::new()
            .get(format!("{DISCORD_URL}/api/users/@me"))
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json::<DiscordUser>()
            .await?)
    }
}

impl DiscordClient {
    pub fn get_authorization_url(&self, callback_url: &str, state: Option<&str>) -> String {
        let scopes = SCOPES.join(" ");
        let mut params: Vec<(&str, &str)> = vec![
            ("client_id", &self.client_id),
            ("response_type", "code"),
            ("redirect_uri", callback_url),
            ("scope", &scopes),
        ];

        if let Some(state) = state {
            params.push(("state", state));
        }

        Url::parse_with_params(&format!("{DISCORD_URL}/oauth2/authorize"), &params)
            .expect("should not fail to parse url")
            .to_string()
    }

    pub async fn exchange_code_for_token(
        &self,
        code: &str,
        callback_url: &str,
    ) -> Result<String, DiscordError> {
        #[derive(Deserialize)]
        struct AccessTokenResponse {
            access_token: String,
        }

        Ok(reqwest::Client::new()
            .post(format!("{DISCORD_URL}/api/oauth2/token"))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", callback_url),
            ])
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .send()
            .await?
            .error_for_status()?
            .json::<AccessTokenResponse>()
            .await?
            .access_token)
    }
}
