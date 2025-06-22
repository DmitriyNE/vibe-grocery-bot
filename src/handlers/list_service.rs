use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId},
};

use super::list::{format_list, format_plain_list};
use crate::db::Database;
use crate::messages::{
    ARCHIVED_LIST_HEADER, CHECKED_ITEMS_ARCHIVED, LIST_ARCHIVED, LIST_EMPTY, LIST_EMPTY_ADD_ITEM,
    LIST_NOW_EMPTY, LIST_NUKED, NO_ACTIVE_LIST_TO_ARCHIVE, NO_CHECKED_ITEMS_TO_ARCHIVE,
};
use crate::utils::{try_delete_message, try_edit_message};

pub struct ListService<'a> {
    db: &'a Database,
}

impl<'a> ListService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub async fn send_list(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
        if let Some(msg_id) = self.db.get_last_list_message_id(chat_id).await? {
            try_delete_message(&bot, chat_id, MessageId(msg_id)).await;
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
            let markup = InlineKeyboardMarkup::new(Vec::<Vec<InlineKeyboardButton>>::new());
            try_edit_message(bot, chat_id, message_id, LIST_NOW_EMPTY, markup).await;
            return Ok(());
        }

        let (text, keyboard) = format_list(&items);
        try_edit_message(bot, chat_id, message_id, text, keyboard).await;
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

        try_delete_message(&bot, chat_id, MessageId(last_message_id)).await;
        bot.send_message(chat_id, archived_text).await?;

        self.db.delete_all_items(chat_id).await?;
        self.db.clear_last_list_message_id(chat_id).await?;

        bot.send_message(chat_id, LIST_ARCHIVED).await?;
        Ok(())
    }

    pub async fn archive_checked(&self, bot: Bot, chat_id: ChatId) -> Result<()> {
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

        let (done, remaining): (Vec<_>, Vec<_>) = items.into_iter().partition(|i| i.done);

        if done.is_empty() {
            bot.send_message(chat_id, NO_CHECKED_ITEMS_TO_ARCHIVE)
                .await?;
            return Ok(());
        }

        if remaining.is_empty() {
            self.archive(bot, chat_id).await?;
            return Ok(());
        }

        tracing::debug!(
            chat_id = chat_id.0,
            done = done.len(),
            remaining = remaining.len(),
            "Archiving checked items"
        );

        let (archived_text, _) = format_list(&done);
        let archived_text = format!("{ARCHIVED_LIST_HEADER}\n{}", archived_text);
        try_delete_message(&bot, chat_id, MessageId(last_message_id)).await;
        bot.send_message(chat_id, archived_text).await?;

        let ids: Vec<i64> = done.iter().map(|i| i.id).collect();
        self.db.delete_items(chat_id, &ids).await?;

        bot.send_message(chat_id, CHECKED_ITEMS_ARCHIVED).await?;

        let (text, keyboard) = format_list(&remaining);
        let sent = bot
            .send_message(chat_id, text)
            .reply_markup(keyboard)
            .await?;
        self.db
            .update_last_list_message_id(chat_id, sent.id)
            .await?;
        Ok(())
    }

    pub async fn nuke(&self, bot: Bot, msg: Message, delete_after_timeout: u64) -> Result<()> {
        try_delete_message(&bot, msg.chat.id, msg.id).await;
        if let Some(list_message_id) = self.db.get_last_list_message_id(msg.chat.id).await? {
            try_delete_message(&bot, msg.chat.id, MessageId(list_message_id)).await;
        }
        self.db.delete_all_items(msg.chat.id).await?;
        self.db.clear_last_list_message_id(msg.chat.id).await?;
        let confirmation = bot.send_message(msg.chat.id, LIST_NUKED).await?;
        drop(crate::delete_after(
            bot.clone(),
            confirmation.chat.id,
            confirmation.id,
            delete_after_timeout,
        ));
        Ok(())
    }
}
