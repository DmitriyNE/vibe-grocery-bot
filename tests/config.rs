use shopbot::ai::config::AiConfig;
use shopbot::Config;

use serial_test::serial;

#[test]
#[serial]
fn ai_config_from_env_missing_key() {
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    assert!(AiConfig::from_env().is_none());
}

#[test]
#[serial]
fn ai_config_from_env_defaults() {
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    std::env::remove_var("OPENAI_CHAT_URL");
    std::env::remove_var("OPENAI_STT_URL");
    let cfg = AiConfig::from_env().unwrap();
    assert_eq!(cfg.api_key, "k");
    assert_eq!(cfg.stt_model, "whisper-1");
    assert_eq!(cfg.gpt_model, "gpt-4.1");
    assert_eq!(cfg.vision_model, "gpt-4o");
    assert!(cfg.openai_chat_url.is_none());
    assert!(cfg.openai_stt_url.is_none());
}

#[test]
#[serial]
fn ai_config_from_env_custom_models() {
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::set_var("OPENAI_STT_MODEL", "s");
    std::env::set_var("OPENAI_GPT_MODEL", "g");
    std::env::set_var("OPENAI_VISION_MODEL", "v");
    std::env::remove_var("OPENAI_CHAT_URL");
    std::env::remove_var("OPENAI_STT_URL");
    let cfg = AiConfig::from_env().unwrap();
    assert_eq!(cfg.stt_model, "s");
    assert_eq!(cfg.gpt_model, "g");
    assert_eq!(cfg.vision_model, "v");
}

#[test]
#[serial]
fn ai_config_from_env_custom_urls() {
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    std::env::set_var("OPENAI_CHAT_URL", "http://chat");
    std::env::set_var("OPENAI_STT_URL", "http://stt");
    let cfg = AiConfig::from_env().unwrap();
    assert_eq!(cfg.openai_chat_url.as_deref(), Some("http://chat"));
    assert_eq!(cfg.openai_stt_url.as_deref(), Some("http://stt"));
}

#[test]
#[serial]
fn config_from_env_calls_ai_constructor() {
    std::env::set_var("DB_URL", "db");
    std::env::remove_var("DB_POOL_SIZE");
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    let cfg = Config::from_env();
    assert_eq!(cfg.db_url, "db");
    assert_eq!(cfg.db_pool_size, 5);
    let ai = cfg.ai.unwrap();
    assert_eq!(ai.api_key, "k");
    assert_eq!(ai.stt_model, "whisper-1");
}

#[test]
#[serial]
fn config_from_env_custom_pool_size() {
    std::env::set_var("DB_URL", "db");
    std::env::set_var("DB_POOL_SIZE", "2");
    std::env::remove_var("OPENAI_API_KEY");
    let cfg = Config::from_env();
    assert_eq!(cfg.db_pool_size, 2);
    assert!(cfg.ai.is_none());
}
