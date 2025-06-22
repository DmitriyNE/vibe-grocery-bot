//! Shared text sent by the bot.
//!
//! Keep all user-facing strings in this module so they stay in one place and are
//! easy to update or translate.

pub const HELP_TEXT: &str =
    "Send me any text to add it to your list. Each line will be a new item.\n\
             You can tap the checkbox button next to an item to mark it as bought.\n\n\
             <b>Commands:</b>\n\
             /list - Show the current list.\n\
             /archive - Finalize and archive the current list, starting a new one.\n\
             /done - Archive only checked items, keeping the rest.\n\
             /delete - Show a temporary panel to delete items from the list.\n\
             /share - Send the list as plain text for copying.\n\
             /nuke - Completely delete the current list.\n\
             /parse - Parse this message into items via GPT.\n\
             /info - Show system information.";

pub const GPT_PARSING_DISABLED: &str = "GPT parsing is disabled.";

pub const NO_ACTIVE_LIST_TO_EDIT: &str = "There is no active list to edit.";
pub const NO_ACTIVE_LIST_TO_ARCHIVE: &str = "There is no active list to archive.";

pub const LIST_EMPTY_ADD_ITEM: &str = "Your list is empty! Send any message to add an item.";
pub const LIST_EMPTY: &str = "Your list is empty!";
pub const LIST_NOW_EMPTY: &str = "List is now empty!";
pub const LIST_ARCHIVED: &str = "List archived! Send a message to start a new one.";
pub const LIST_NUKED: &str = "The active list has been nuked.";
pub const CHECKED_ITEMS_ARCHIVED: &str = "Checked items archived!";
pub const NO_CHECKED_ITEMS_TO_ARCHIVE: &str = "There are no checked items to archive.";

pub const DELETE_SELECT_PROMPT: &str = "Select items to delete, then tap 'Done Deleting'.";
pub const DELETE_DONE_LABEL: &str = "🗑️ Done Deleting";
pub fn delete_dm_text(chat_name: &str, list_text: &str) -> String {
    format!("Deleting items from {chat_name}.\n\n{list_text}")
}

pub fn delete_user_selecting_text(user_name: &str) -> String {
    format!("{user_name} is selecting items to delete...")
}
pub const DELETE_DM_FAILED: &str =
    "Unable to send you a private delete panel. Have you started me in private?";
pub const DEFAULT_CHAT_NAME: &str = "your list";

pub const ARCHIVED_LIST_HEADER: &str = "--- Archived List ---";
pub const VOICE_REMOVED_PREFIX: &str = "🗑 Removed via voice request:\n";
