use crate::db::Database;
use anyhow::Result;
use std::collections::HashSet;
use teloxide::{
    prelude::*,
    types::{
        ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MaybeInaccessibleMessage, MessageId,
        User, UserId,
    },
};

use crate::db::Item;
use crate::messages::{
    delete_dm_text, delete_user_selecting_text, DEFAULT_CHAT_NAME, DELETE_DM_FAILED,
    DELETE_DONE_LABEL, DELETE_SELECT_PROMPT, NO_ACTIVE_LIST_TO_EDIT,
};

use super::list::update_message;
use crate::utils::{try_delete_message, try_edit_message};

pub fn format_delete_list(
    items: &[Item],
    selected: &HashSet<i64>,
) -> (String, InlineKeyboardMarkup) {
    let text = DELETE_SELECT_PROMPT.to_string();

    let mut keyboard_buttons = Vec::new();

    for item in items {
        let button_text = if selected.contains(&item.id) {
            format!("❌ {}", item.text)
        } else {
            format!("⬜ {}", item.text)
        };
        let callback_data = format!("delete_{}", item.id);
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            button_text,
            callback_data,
        )]);
    }

    keyboard_buttons.push(vec![InlineKeyboardButton::callback(
        DELETE_DONE_LABEL,
        "delete_done",
    )]);

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

async fn cleanup_previous_session(bot: &Bot, db: &Database, user_id: UserId) -> Result<()> {
    tracing::debug!(user_id = user_id.0, "Cleaning up previous delete session");
    if let Some(prev) = db.get_delete_session(user_id.0 as i64).await? {
        if let Some((chat_id, msg_id)) = prev.notice {
            try_delete_message(bot, chat_id, msg_id).await;
        }
        if let Some(dm) = prev.dm_message_id {
            try_delete_message(bot, ChatId(user_id.0 as i64), dm).await;
        }
    }
    Ok(())
}

async fn start_delete_session(
    bot: &Bot,
    msg: &Message,
    user: &User,
    db: &Database,
    items: &[Item],
    delete_after_timeout: u64,
) -> Result<()> {
    tracing::debug!(
        chat_id = msg.chat.id.0,
        user_id = user.id.0,
        "Starting delete session",
    );

    db.init_delete_session(user.id.0 as i64, msg.chat.id)
        .await?;

    let (base_text, keyboard) = format_delete_list(items, &HashSet::new());
    let chat_name = msg
        .chat
        .title()
        .map(ToString::to_string)
        .unwrap_or_else(|| DEFAULT_CHAT_NAME.to_string());
    let dm_text = delete_dm_text(&chat_name, &base_text);

    match bot
        .send_message(UserId(user.id.0), dm_text)
        .reply_markup(keyboard)
        .await
    {
        Ok(dm_msg) => {
            db.set_delete_dm_message(user.id.0 as i64, dm_msg.id)
                .await?;
            if !msg.chat.is_private() {
                let info = bot
                    .send_message(msg.chat.id, delete_user_selecting_text(&user.first_name))
                    .await?;
                db.set_delete_notice(user.id.0 as i64, msg.chat.id, info.id)
                    .await?;
            }
        }
        Err(err) => {
            tracing::warn!("failed to send DM: {}", err);
            let warn = bot.send_message(msg.chat.id, DELETE_DM_FAILED).await?;
            drop(crate::delete_after(
                bot.clone(),
                warn.chat.id,
                warn.id,
                delete_after_timeout,
            ));
        }
    }

    Ok(())
}

async fn process_done_callback(
    bot: &Bot,
    msg: &MaybeInaccessibleMessage,
    user_id: i64,
    db: &Database,
) -> Result<()> {
    if let Some(session) = db.get_delete_session(user_id).await? {
        if session.dm_message_id.map(|m| m.0) != Some(msg.id().0) {
            return Ok(());
        }
        for id in &session.selected {
            db.delete_item(session.chat_id, *id).await?;
        }
        if let Some(main_list_id) = db.get_last_list_message_id(session.chat_id).await? {
            update_message(bot, session.chat_id, MessageId(main_list_id), db).await?;
        }
        if let Some((chat_id, notice_id)) = session.notice {
            try_delete_message(bot, chat_id, notice_id).await;
        }
        db.clear_delete_session(user_id).await?;
    }
    try_delete_message(bot, msg.chat().id, msg.id()).await;
    Ok(())
}

async fn toggle_selection(
    bot: &Bot,
    msg: &MaybeInaccessibleMessage,
    user_id: i64,
    id: i64,
    db: &Database,
) -> Result<()> {
    if let Some(mut session) = db.get_delete_session(user_id).await? {
        if session.dm_message_id.map(|m| m.0) != Some(msg.id().0) {
            return Ok(());
        }
        if session.selected.contains(&id) {
            session.selected.remove(&id);
        } else {
            session.selected.insert(id);
        }
        db.update_delete_selection(user_id, &session.selected)
            .await?;
        let items = db.list_items(session.chat_id).await?;
        let (text, keyboard) = format_delete_list(&items, &session.selected);
        try_edit_message(bot, msg.chat().id, msg.id(), text, keyboard).await;
    }
    Ok(())
}

pub async fn enter_delete_mode(
    bot: Bot,
    msg: Message,
    db: &Database,
    delete_after_timeout: u64,
) -> Result<()> {
    tracing::debug!(
        chat_id = msg.chat.id.0,
        user_id = msg.from.as_ref().map(|u| u.id.0),
        "Entering delete mode",
    );
    try_delete_message(&bot, msg.chat.id, msg.id).await;

    if db.get_last_list_message_id(msg.chat.id).await?.is_none() {
        let sent_msg = bot
            .send_message(msg.chat.id, NO_ACTIVE_LIST_TO_EDIT)
            .await?;
        drop(crate::delete_after(
            bot.clone(),
            sent_msg.chat.id,
            sent_msg.id,
            delete_after_timeout,
        ));
        return Ok(());
    }

    let user = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    cleanup_previous_session(&bot, db, user.id).await?;

    let items = db.list_items(msg.chat.id).await?;
    if items.is_empty() {
        return Ok(());
    }

    start_delete_session(&bot, &msg, user, db, &items, delete_after_timeout).await
}

pub async fn callback_handler(bot: Bot, q: CallbackQuery, db: Database) -> Result<()> {
    if let (Some(data), Some(msg)) = (q.data, q.message) {
        if let Some(id_str) = data.strip_prefix("delete_") {
            let user_id = q.from.id.0 as i64;

            if id_str == "done" {
                process_done_callback(&bot, &msg, user_id, &db).await?;
            } else if let Ok(id) = id_str.parse::<i64>() {
                toggle_selection(&bot, &msg, user_id, id, &db).await?;
            }
        } else if let Ok(id) = data.parse::<i64>() {
            db.toggle_item(msg.chat().id, id).await?;
            update_message(&bot, msg.chat().id, msg.id(), &db).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::util::init_test_db;
    use teloxide::types::{ChatId, MaybeInaccessibleMessage, MessageId, UserId};
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn cleanup_previous_session_deletes_messages() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/botTEST/DeleteMessage"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
            )
            .expect(2)
            .mount(&server)
            .await;

        let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
        let db = init_test_db().await;
        let user = UserId(1);
        db.init_delete_session(user.0 as i64, ChatId(1))
            .await
            .unwrap();
        db.set_delete_notice(user.0 as i64, ChatId(1), MessageId(10))
            .await
            .unwrap();
        db.set_delete_dm_message(user.0 as i64, MessageId(11))
            .await
            .unwrap();

        cleanup_previous_session(&bot, &db, user).await.unwrap();
        server.verify().await;
    }

    #[tokio::test]
    async fn toggle_selection_updates_db() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/botTEST/EditMessageText"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"{"ok":true,"result":true}"#, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let bot = Bot::new("TEST").set_api_url(reqwest::Url::parse(&server.uri()).unwrap());
        let db = init_test_db().await;
        let chat = ChatId(1);
        db.add_item(chat, "Milk").await.unwrap();
        let items = db.list_items(chat).await.unwrap();
        let item_id = items[0].id;

        db.init_delete_session(1, chat).await.unwrap();
        db.set_delete_dm_message(1, MessageId(5)).await.unwrap();
        let msg_json = r#"{"message_id":5,"date":0,"chat":{"id":1,"type":"private"}}"#;
        let msg: MaybeInaccessibleMessage = serde_json::from_str(msg_json).unwrap();

        toggle_selection(&bot, &msg, 1, item_id, &db).await.unwrap();
        let session = db.get_delete_session(1).await.unwrap().unwrap();
        assert!(session.selected.contains(&item_id));
        server.verify().await;
    }
}
