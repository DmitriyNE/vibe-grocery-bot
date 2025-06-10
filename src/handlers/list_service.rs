use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId},
};

use super::list::{format_list, format_plain_list};
use crate::db::Database;

pub struct ListService<'a> {
    db: &'a Database,
}

impl<'a> ListService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub async fn send_list(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
        if let Some(msg_id) = self.db.get_last_list_message_id(chat_id).await? {
            let _ = bot.delete_message(chat_id, MessageId(msg_id)).await;
        }

        let items = self.db.list_items(chat_id).await?;
        if items.is_empty() {
            let sent = bot
                .send_message(
                    chat_id,
                    "Your shopping list is empty! Send any message to add an item.",
                )
                .await?;
            self.db
                .update_last_list_message_id(chat_id, sent.id)
                .await?;
            return Ok(());
        }

        let (text, keyboard) = format_list(&items);
        let sent = bot
            .send_message(chat_id, text)
            .reply_markup(keyboard)
            .await?;
        self.db
            .update_last_list_message_id(chat_id, sent.id)
            .await?;
        Ok(())
    }

    pub async fn share_list(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
        let items = self.db.list_items(chat_id).await?;
        if items.is_empty() {
            bot.send_message(chat_id, "Your shopping list is empty!")
                .await?;
            return Ok(());
        }
        let text = format_plain_list(&items);
        bot.send_message(chat_id, text).await?;
        Ok(())
    }

    pub async fn update_message(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        message_id: MessageId,
    ) -> Result<()> {
        let items = self.db.list_items(chat_id).await?;
        if items.is_empty() {
            let _ = bot
                .edit_message_text(chat_id, message_id, "List is now empty!")
                .reply_markup(InlineKeyboardMarkup::new(
                    Vec::<Vec<InlineKeyboardButton>>::new(),
                ))
                .await;
            return Ok(());
        }

        let (text, keyboard) = format_list(&items);
        let _ = bot
            .edit_message_text(chat_id, message_id, text)
            .reply_markup(keyboard)
            .await;
        Ok(())
    }

    pub async fn archive(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
        let last_message_id = match self.db.get_last_list_message_id(chat_id).await? {
            Some(id) => id,
            None => {
                bot.send_message(chat_id, "There is no active list to archive.")
                    .await?;
                return Ok(());
            }
        };

        let items = self.db.list_items(chat_id).await?;
        if items.is_empty() {
            bot.send_message(chat_id, "There is no active list to archive.")
                .await?;
            return Ok(());
        }

        let (final_text, _) = format_list(&items);
        let archived_text = format!("--- Archived List ---\n{}", final_text);

        let _ = bot
            .edit_message_text(chat_id, MessageId(last_message_id), archived_text)
            .reply_markup(InlineKeyboardMarkup::new(
                Vec::<Vec<InlineKeyboardButton>>::new(),
            ))
            .await;

        self.db.delete_all_items(chat_id).await?;
        self.db.clear_last_list_message_id(chat_id).await?;

        bot.send_message(chat_id, "List archived! Send a message to start a new one.")
            .await?;
        Ok(())
    }

    pub async fn nuke(&self, bot: Bot, msg: Message) -> Result<()> {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        if let Some(list_message_id) = self.db.get_last_list_message_id(msg.chat.id).await? {
            let _ = bot
                .delete_message(msg.chat.id, MessageId(list_message_id))
                .await;
        }
        self.db.delete_all_items(msg.chat.id).await?;
        self.db.clear_last_list_message_id(msg.chat.id).await?;
        let confirmation = bot
            .send_message(msg.chat.id, "The active list has been nuked.")
            .await?;
        crate::delete_after(bot.clone(), confirmation.chat.id, confirmation.id, 5);
        Ok(())
    }
}
