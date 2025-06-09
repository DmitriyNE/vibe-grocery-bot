// Database related types and functions

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

pub mod chat_state;
pub mod delete_session;
pub mod items;

pub use chat_state::{
    clear_last_list_message_id, get_last_list_message_id, update_last_list_message_id,
};
#[allow(unused_imports)]
pub use delete_session::DeleteSession;
pub use delete_session::{
    clear_delete_session, get_delete_session, init_delete_session, set_delete_dm_message,
    set_delete_notice, update_delete_selection,
};
pub use items::{
    add_item, delete_all_items, delete_item, delete_items, list_items, toggle_item, Item,
};

pub fn prepare_sqlite_url(url: &str) -> String {
    if url.starts_with("sqlite:") && !url.contains("mode=") && !url.contains(":memory:") {
        if url.contains('?') {
            format!("{url}&mode=rwc")
        } else {
            format!("{url}?mode=rwc")
        }
    } else {
        url.to_string()
    }
}

pub async fn connect_db(db_url: &str) -> Result<Pool<Sqlite>> {
    tracing::debug!(db_url = %db_url, "Connecting to database");
    Ok(SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_sqlite_url_basic() {
        assert_eq!(
            prepare_sqlite_url("sqlite:items.db"),
            "sqlite:items.db?mode=rwc"
        );
    }

    #[test]
    fn prepare_sqlite_url_with_query() {
        assert_eq!(
            prepare_sqlite_url("sqlite:items.db?cache=shared"),
            "sqlite:items.db?cache=shared&mode=rwc"
        );
    }

    #[test]
    fn prepare_sqlite_url_existing_mode() {
        assert_eq!(
            prepare_sqlite_url("sqlite:items.db?mode=ro"),
            "sqlite:items.db?mode=ro"
        );
    }

    #[test]
    fn prepare_sqlite_url_memory() {
        assert_eq!(prepare_sqlite_url("sqlite::memory:"), "sqlite::memory:");
    }
}
