use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "show the current shopping list.")]
    List,
    #[command(description = "finalize and archive the current list, starting a new one.")]
    Archive,
    #[command(description = "show a temporary panel to delete items from the list.")]
    Delete,
    #[command(description = "send the list as plain text for copying.")]
    Share,
    #[command(description = "completely delete the current list.")]
    Nuke,
    #[command(description = "parse items from the given text using GPT.")]
    Parse,
    #[command(description = "show system information.")]
    Info,
}
