use shopbot::ai::config::AiConfig;
use shopbot::Config;

#[test]
fn ai_config_from_env_missing_key() {
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    assert!(AiConfig::from_env().is_none());
}

#[test]
fn ai_config_from_env_defaults() {
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    let cfg = AiConfig::from_env().unwrap();
    assert_eq!(cfg.api_key, "k");
    assert_eq!(cfg.stt_model, "whisper-1");
    assert_eq!(cfg.gpt_model, "gpt-4.1");
    assert_eq!(cfg.vision_model, "gpt-4o");
}

#[test]
fn ai_config_from_env_custom_models() {
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::set_var("OPENAI_STT_MODEL", "s");
    std::env::set_var("OPENAI_GPT_MODEL", "g");
    std::env::set_var("OPENAI_VISION_MODEL", "v");
    let cfg = AiConfig::from_env().unwrap();
    assert_eq!(cfg.stt_model, "s");
    assert_eq!(cfg.gpt_model, "g");
    assert_eq!(cfg.vision_model, "v");
}

#[test]
fn config_from_env_calls_ai_constructor() {
    std::env::set_var("DB_URL", "db");
    std::env::set_var("OPENAI_API_KEY", "k");
    std::env::remove_var("OPENAI_STT_MODEL");
    std::env::remove_var("OPENAI_GPT_MODEL");
    std::env::remove_var("OPENAI_VISION_MODEL");
    let cfg = Config::from_env();
    assert_eq!(cfg.db_url, "db");
    let ai = cfg.ai.unwrap();
    assert_eq!(ai.api_key, "k");
    assert_eq!(ai.stt_model, "whisper-1");
}
