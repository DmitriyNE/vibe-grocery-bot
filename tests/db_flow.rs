use anyhow::Result;
use shopbot::db::{ChatKey, ItemId};
use shopbot::tests::util::init_test_db;
use teloxide::types::{ChatId, MessageId};

#[tokio::test]
async fn basic_item_flow() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(42);
    let key = ChatKey(chat.0);

    db.add_item(key, "Apples").await?;
    db.add_item(key, "Milk").await?;

    let mut items = db.list_items(key).await?;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].text, "Apples");
    assert!(!items[0].done);

    db.toggle_item(key, items[0].id).await?;
    items = db.list_items(key).await?;
    assert!(items[0].done);

    db.delete_item(key, items[1].id).await?;
    items = db.list_items(key).await?;
    assert_eq!(items.len(), 1);

    db.delete_all_items(key).await?;
    items = db.list_items(key).await?;
    assert!(items.is_empty());

    Ok(())
}

#[tokio::test]
async fn last_message_id_roundtrip() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(1);
    let key = ChatKey(chat.0);

    assert!(db.get_last_list_message_id(key).await?.is_none());

    db.update_last_list_message_id(key, MessageId(99)).await?;
    assert_eq!(db.get_last_list_message_id(key).await?, Some(99));

    db.clear_last_list_message_id(key).await?;
    assert!(db.get_last_list_message_id(key).await?.is_none());

    Ok(())
}

#[tokio::test]
async fn delete_multiple_items() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(2);
    let key = ChatKey(chat.0);
    for i in 0..3 {
        db.add_item(key, &format!("Item {i}")).await?;
    }

    let items = db.list_items(key).await?;
    let ids: Vec<ItemId> = items.iter().map(|i| i.id).collect();

    db.delete_items(key, &ids).await?;

    let remaining = db.list_items(key).await?;
    assert!(remaining.is_empty());

    Ok(())
}
