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
        let ai = match env::var("OPENAI_API_KEY") {
            Ok(key) => Some(AiConfig {
                api_key: key,
                stt_model: env::var("OPENAI_STT_MODEL").unwrap_or_else(|_| "whisper-1".to_string()),
                gpt_model: env::var("OPENAI_GPT_MODEL").unwrap_or_else(|_| "gpt-4.1".to_string()),
                vision_model: env::var("OPENAI_VISION_MODEL")
                    .unwrap_or_else(|_| "gpt-4o".to_string()),
            }),
            Err(_) => None,
        };
        Self { db_url, ai }
    }
}
