use anyhow::Result;
use shopbot::db::*;
use shopbot::tests::util::init_test_db;
use teloxide::types::{ChatId, MessageId};

#[tokio::test]
async fn basic_item_flow() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(42);

    add_item(&db, chat, "Apples").await?;
    add_item(&db, chat, "Milk").await?;

    let mut items = list_items(&db, chat).await?;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].text, "Apples");
    assert!(!items[0].done);

    toggle_item(&db, items[0].id).await?;
    items = list_items(&db, chat).await?;
    assert!(items[0].done);

    delete_item(&db, items[1].id).await?;
    items = list_items(&db, chat).await?;
    assert_eq!(items.len(), 1);

    delete_all_items(&db, chat).await?;
    items = list_items(&db, chat).await?;
    assert!(items.is_empty());

    Ok(())
}

#[tokio::test]
async fn last_message_id_roundtrip() -> Result<()> {
    let db = init_test_db().await;
    let chat = ChatId(1);

    assert!(get_last_list_message_id(&db, chat).await?.is_none());

    update_last_list_message_id(&db, chat, MessageId(99)).await?;
    assert_eq!(get_last_list_message_id(&db, chat).await?, Some(99));

    clear_last_list_message_id(&db, chat).await?;
    assert!(get_last_list_message_id(&db, chat).await?.is_none());

    Ok(())
}
