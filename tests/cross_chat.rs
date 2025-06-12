use shopbot::db::{ChatKey, ItemId};
use shopbot::tests::util::init_test_db;
use teloxide::types::ChatId;

#[tokio::test]
async fn toggle_different_chat_has_no_effect() {
    let db = init_test_db().await;
    let chat1 = ChatId(1);
    let chat2 = ChatId(2);
    db.add_item(ChatKey(chat1.0), "Milk").await.unwrap();

    let item = db.list_items(ChatKey(chat1.0)).await.unwrap()[0].clone();
    db.toggle_item(ChatKey(chat2.0), item.id).await.unwrap();

    let after = db.list_items(ChatKey(chat1.0)).await.unwrap()[0].clone();
    assert!(!after.done);
}

#[tokio::test]
async fn delete_different_chat_has_no_effect() {
    let db = init_test_db().await;
    let chat1 = ChatId(1);
    let chat2 = ChatId(2);
    db.add_item(ChatKey(chat1.0), "Milk").await.unwrap();
    let item = db.list_items(ChatKey(chat1.0)).await.unwrap()[0].clone();

    db.delete_item(ChatKey(chat2.0), item.id).await.unwrap();
    let remaining = db.list_items(ChatKey(chat1.0)).await.unwrap();
    assert_eq!(remaining.len(), 1);
}

#[tokio::test]
async fn delete_multiple_different_chat_has_no_effect() {
    let db = init_test_db().await;
    let chat1 = ChatId(1);
    let chat2 = ChatId(2);
    for _ in 0..3 {
        db.add_item(ChatKey(chat1.0), "Item").await.unwrap();
    }
    let items = db.list_items(ChatKey(chat1.0)).await.unwrap();
    let ids: Vec<ItemId> = items.iter().map(|i| i.id).collect();

    db.delete_items(ChatKey(chat2.0), &ids).await.unwrap();
    let remaining = db.list_items(ChatKey(chat1.0)).await.unwrap();
    assert_eq!(remaining.len(), 3);
}
