//! Common system prompts used by the AI helpers.
//!
//! Centralizing these strings makes it easy to tweak how text, photos
//! and audio are interpreted without digging through multiple modules.

/// System prompt for parsing items from free-form text.
pub const TEXT_PARSING_PROMPT: &str = "Extract the items from the user's text. Use the nominative form for nouns when it does not change the meaning. Convert number words to digits so 'три ананаса' becomes '3 ананаса'. Respond with a JSON object like {\"items\": [\"1 milk\"]}";

/// System prompt for parsing items from a photo.
pub const PHOTO_PARSING_PROMPT: &str = "Extract the items shown in the photo. Respond with a JSON object like {\"items\": [\"apples\"]}.";

/// Default instructions passed to GPT-based transcription models.
/// The prompt also asks the model to keep verbs intact so commands like
/// "delete" are not dropped during transcription. Quantities should be
/// written using digits when possible. Convert spelled-out numbers to digits
/// so phrases like "три ананаса" become "3 ананаса".
pub const DEFAULT_STT_PROMPT: &str = "Transcribe the user's request about the list. Keep verbs like 'add' or 'delete' exactly as spoken. Use digits for quantities and convert number words to digits.";
