use rand::rngs::StdRng;
use tokio::sync::Mutex;

use crate::config::WebConfig;

pub mod api;
pub mod app;

pub struct WebState {
    pub config: WebConfig,
    pub rng: Mutex<StdRng>,
}
