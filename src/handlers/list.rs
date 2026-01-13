use crate::db::{Database, Item};
use crate::text_utils::capitalize_first;
use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup},
};

use super::list_service::ListService;

pub fn format_list(items: &[Item]) -> (String, InlineKeyboardMarkup) {
    let mut text = String::new();
    let mut keyboard_buttons = Vec::new();

    let all_done = items.iter().all(|i| i.done);

    for item in items {
        let (_mark, label) = format_item_entry(item, all_done);
        text.push_str(&label);
        text.push('\n');
        keyboard_buttons.push(vec![InlineKeyboardButton::callback(
            label,
            item.id.to_string(),
        )]);
    }

    if all_done && !items.is_empty() {
        tracing::debug!("List fully checked out");
    }

    (text, InlineKeyboardMarkup::new(keyboard_buttons))
}

pub fn format_plain_list(items: &[Item]) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(&format!("• {}\n", item.text));
    }
    text
}

fn capitalize_items<I>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    items
        .into_iter()
        .map(|item| capitalize_first(&item))
        .collect()
}

fn format_item_entry(item: &Item, all_done: bool) -> (&'static str, String) {
    let mark = if all_done {
        "✅"
    } else if item.done {
        "☑️"
    } else {
        "⬜"
    };
    let label = format!("{mark} {}", item.text);
    (mark, label)
}

pub async fn insert_items<I>(bot: Bot, chat_id: ChatId, db: &Database, items: I) -> Result<usize>
where
    I: IntoIterator<Item = String>,
{
    let mut added = 0usize;
    for item in items {
        db.add_item(chat_id, &item).await?;
        added += 1;
    }

    if added > 0 {
        tracing::debug!(chat_id = chat_id.0, added, "Inserted items");
        ListService::new(db).send_list(bot, chat_id).await?;
    } else {
        tracing::debug!(chat_id = chat_id.0, "No items inserted");
    }
    Ok(added)
}

pub async fn insert_capitalized_items_with_log<I>(
    bot: Bot,
    chat_id: ChatId,
    db: &Database,
    items: I,
    context: &str,
) -> Result<usize>
where
    I: IntoIterator<Item = String>,
{
    let items = capitalize_items(items);
    let added = insert_items(bot, chat_id, db, items).await?;
    if added > 0 {
        tracing::info!(chat_id = chat_id.0, added, context, "Added items");
    }
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::{capitalize_items, format_item_entry};
    use crate::db::Item;

    #[test]
    fn capitalize_items_preserves_sequences() {
        let items = vec!["apple".to_string(), "Éclair".to_string()];
        let capitalized = capitalize_items(items);
        assert_eq!(capitalized, vec!["Apple".to_string(), "Éclair".to_string()]);
    }

    #[test]
    fn format_item_entry_marks_incomplete_items() {
        let item = Item {
            id: 1,
            text: "Milk".to_string(),
            done: false,
        };
        let (mark, label) = format_item_entry(&item, false);
        assert_eq!(mark, "⬜");
        assert_eq!(label, "⬜ Milk");
    }

    #[test]
    fn format_item_entry_marks_checked_items() {
        let item = Item {
            id: 2,
            text: "Eggs".to_string(),
            done: true,
        };
        let (mark, label) = format_item_entry(&item, false);
        assert_eq!(mark, "☑️");
        assert_eq!(label, "☑️ Eggs");
    }

    #[test]
    fn format_item_entry_marks_all_done_items() {
        let item = Item {
            id: 3,
            text: "Bread".to_string(),
            done: true,
        };
        let (mark, label) = format_item_entry(&item, true);
        assert_eq!(mark, "✅");
        assert_eq!(label, "✅ Bread");
    }
}
