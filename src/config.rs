use std::env;

use crate::ai::config::AiConfig;

#[derive(Clone)]
pub struct Config {
    pub db_url: String,
    pub db_pool_size: u32,
    pub ai: Option<AiConfig>,
    pub delete_after_timeout: u64,
    pub api_bind_addr: String,
    pub api_rate_limit_per_second: Option<u64>,
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
        let api_bind_addr =
            env::var("API_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let api_rate_limit_per_second = env::var("API_RATE_LIMIT_PER_SECOND")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|value| *value > 0);
        let ai = AiConfig::from_env();
        Self {
            db_url,
            db_pool_size,
            ai,
            delete_after_timeout,
            api_bind_addr,
            api_rate_limit_per_second,
        }
    }
}
