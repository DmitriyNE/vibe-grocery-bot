pub use teloxide::types::ChatId;

/// Identifier for a chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatKey(pub i64);

/// Identifier for an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub i64);

impl std::fmt::Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ChatId> for ChatKey {
    fn from(id: ChatId) -> Self {
        ChatKey(id.0)
    }
}

impl From<ChatKey> for ChatId {
    fn from(key: ChatKey) -> Self {
        ChatId(key.0)
    }
}

impl From<i64> for ChatKey {
    fn from(id: i64) -> Self {
        ChatKey(id)
    }
}

impl From<ChatKey> for i64 {
    fn from(key: ChatKey) -> Self {
        key.0
    }
}

impl From<i64> for ItemId {
    fn from(id: i64) -> Self {
        ItemId(id)
    }
}

impl From<ItemId> for i64 {
    fn from(id: ItemId) -> Self {
        id.0
    }
}
