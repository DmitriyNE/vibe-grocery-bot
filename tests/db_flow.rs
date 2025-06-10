use anyhow::Result;
use shopbot::tests::util::init_test_db;
use teloxide::types::{ChatId, MessageId};

#[tokio::test]
async fn basic_item_flow() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(42);

    db.add_item(chat, "Apples").await?;
    db.add_item(chat, "Milk").await?;

    let mut items = db.list_items(chat).await?;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].text, "Apples");
    assert!(!items[0].done);

    db.toggle_item(chat, items[0].id).await?;
    items = db.list_items(chat).await?;
    assert!(items[0].done);

    db.delete_item(chat, items[1].id).await?;
    items = db.list_items(chat).await?;
    assert_eq!(items.len(), 1);

    db.delete_all_items(chat).await?;
    items = db.list_items(chat).await?;
    assert!(items.is_empty());

    Ok(())
}

#[tokio::test]
async fn last_message_id_roundtrip() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(1);

    assert!(db.get_last_list_message_id(chat).await?.is_none());

    db.update_last_list_message_id(chat, MessageId(99)).await?;
    assert_eq!(db.get_last_list_message_id(chat).await?, Some(99));

    db.clear_last_list_message_id(chat).await?;
    assert!(db.get_last_list_message_id(chat).await?.is_none());

    Ok(())
}
