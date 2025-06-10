use std::env;

use crate::ai::config::AiConfig;

#[derive(Clone)]
pub struct Config {
    pub db_url: String,
    pub db_pool_size: u32,
    pub ai: Option<AiConfig>,
    pub delete_after_timeout: u64,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let db_url = env::var("DB_URL").unwrap_or_else(|_| "sqlite:items.db".to_string());
        let db_pool_size = env::var("DB_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(5);
        let delete_after_timeout = env::var("DELETE_AFTER_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(crate::utils::DEFAULT_DELETE_AFTER_TIMEOUT);
        let ai = AiConfig::from_env();
        Self {
            db_url,
            db_pool_size,
            ai,
            delete_after_timeout,
        }
    }
}
