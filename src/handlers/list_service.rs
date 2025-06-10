use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId},
};

use super::list::{format_list, format_plain_list};
use crate::db::Database;
use crate::messages::{
    ARCHIVED_LIST_HEADER, LIST_ARCHIVED, LIST_EMPTY, LIST_EMPTY_ADD_ITEM, LIST_NOW_EMPTY,
    LIST_NUKED, NO_ACTIVE_LIST_TO_ARCHIVE,
};

pub struct ListService<'a> {
    db: &'a Database,
}

impl<'a> ListService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub async fn send_list(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
        if let Some(msg_id) = self.db.get_last_list_message_id(chat_id).await? {
            if let Err(err) = bot.delete_message(chat_id, MessageId(msg_id)).await {
                tracing::warn!(
                    error = %err,
                    chat_id = chat_id.0,
                    message_id = msg_id,
                    "Failed to delete message",
                );
            }
        }

        let items = self.db.list_items(chat_id).await?;
        if items.is_empty() {
            let sent = bot.send_message(chat_id, LIST_EMPTY_ADD_ITEM).await?;
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
            bot.send_message(chat_id, LIST_EMPTY).await?;
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
            if let Err(err) = bot
                .edit_message_text(chat_id, message_id, LIST_NOW_EMPTY)
                .reply_markup(InlineKeyboardMarkup::new(
                    Vec::<Vec<InlineKeyboardButton>>::new(),
                ))
                .await
            {
                tracing::warn!(
                    error = %err,
                    chat_id = chat_id.0,
                    message_id = message_id.0,
                    "Failed to edit message",
                );
            }
            return Ok(());
        }

        let (text, keyboard) = format_list(&items);
        if let Err(err) = bot
            .edit_message_text(chat_id, message_id, text)
            .reply_markup(keyboard)
            .await
        {
            tracing::warn!(
                error = %err,
                chat_id = chat_id.0,
                message_id = message_id.0,
                "Failed to edit message",
            );
        }
        Ok(())
    }

    pub async fn archive(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
        let last_message_id = match self.db.get_last_list_message_id(chat_id).await? {
            Some(id) => id,
            None => {
                bot.send_message(chat_id, NO_ACTIVE_LIST_TO_ARCHIVE).await?;
                return Ok(());
            }
        };

        let items = self.db.list_items(chat_id).await?;
        if items.is_empty() {
            bot.send_message(chat_id, NO_ACTIVE_LIST_TO_ARCHIVE).await?;
            return Ok(());
        }

        let (final_text, _) = format_list(&items);
        let archived_text = format!("{ARCHIVED_LIST_HEADER}\n{}", final_text);

        if let Err(err) = bot
            .edit_message_text(chat_id, MessageId(last_message_id), archived_text)
            .reply_markup(InlineKeyboardMarkup::new(
                Vec::<Vec<InlineKeyboardButton>>::new(),
            ))
            .await
        {
            tracing::warn!(
                error = %err,
                chat_id = chat_id.0,
                message_id = last_message_id,
                "Failed to edit message",
            );
        }

        self.db.delete_all_items(chat_id).await?;
        self.db.clear_last_list_message_id(chat_id).await?;

        bot.send_message(chat_id, LIST_ARCHIVED).await?;
        Ok(())
    }

    pub async fn nuke(&self, bot: Bot, msg: Message) -> Result<()> {
        if let Err(err) = bot.delete_message(msg.chat.id, msg.id).await {
            tracing::warn!(
                error = %err,
                chat_id = msg.chat.id.0,
                message_id = msg.id.0,
                "Failed to delete message",
            );
        }
        if let Some(list_message_id) = self.db.get_last_list_message_id(msg.chat.id).await? {
            if let Err(err) = bot
                .delete_message(msg.chat.id, MessageId(list_message_id))
                .await
            {
                tracing::warn!(
                    error = %err,
                    chat_id = msg.chat.id.0,
                    message_id = list_message_id,
                    "Failed to delete message",
                );
            }
        }
        self.db.delete_all_items(msg.chat.id).await?;
        self.db.clear_last_list_message_id(msg.chat.id).await?;
        let confirmation = bot.send_message(msg.chat.id, LIST_NUKED).await?;
        drop(crate::delete_after(
            bot.clone(),
            confirmation.chat.id,
            confirmation.id,
            crate::utils::DELETE_AFTER_TIMEOUT,
        ));
        Ok(())
    }
}
