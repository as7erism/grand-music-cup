use url::Url;

pub const SCOPES: [&str; 1] = ["identify"];
pub const DISCORD_URL: &str = "https://discord.com";

pub struct DiscordClient {
    pub client_id: String,
    pub client_secret: String,
}

impl DiscordClient {
    pub fn authorization_url(&self, callback_url: &str, state: Option<&str>) -> String {
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
}

fn build_discord_authorization_url(
    client_id: &str,
    callback_url: &str,
    state: Option<&str>,
) -> String {
    let scopes = SCOPES.join(" ");
    let mut params = vec![
        ("client_id", client_id),
        ("response_type", "code"),
        ("redirect_uri", callback_url),
        ("scope", &scopes),
    ];

    if let Some(state) = state {
        params.push(("state", state));
    }

        Url::parse_with_params(&format!("{DISCORD_URL}/oauth2/authorize"), &params)
            .unwrap()
            .to_string()
}
