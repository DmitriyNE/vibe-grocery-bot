use std::env;

use crate::ai::config::AiConfig;

#[derive(Clone)]
pub struct Config {
    pub db_url: String,
    pub ai: Option<AiConfig>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let db_url = env::var("DB_URL").unwrap_or_else(|_| "sqlite:shopping.db".to_string());
        let ai = AiConfig::from_env();
        Self { db_url, ai }
    }
}
