use std::env;
#[derive(Clone)]
pub struct AiConfig {
    pub api_key: String,
    pub stt_model: String,
    pub gpt_model: String,
    pub vision_model: String,
    pub openai_chat_url: Option<String>,
    pub openai_stt_url: Option<String>,
}

impl AiConfig {
    pub fn from_env() -> Option<Self> {
        let api_key = match env::var("OPENAI_API_KEY") {
            Ok(k) => k,
            Err(_) => return None,
        };
        Some(Self {
            api_key,
            stt_model: env::var("OPENAI_STT_MODEL").unwrap_or_else(|_| "whisper-1".to_string()),
            gpt_model: env::var("OPENAI_GPT_MODEL").unwrap_or_else(|_| "gpt-4.1".to_string()),
            vision_model: env::var("OPENAI_VISION_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            openai_chat_url: env::var("OPENAI_CHAT_URL").ok(),
            openai_stt_url: env::var("OPENAI_STT_URL").ok(),
        })
    }
}
