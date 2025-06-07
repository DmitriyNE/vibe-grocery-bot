# Instructions for Codex agents

- Always run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all --no-fail-fast` before committing.
- Use clear commit messages that describe the change.
- Avoid global state. Persist temporary data such as deletion selections and any notice messages in the database.
- Keep the `migrations/` directory up to date whenever the database schema changes so that embedded migrations remain in sync.

# Prohibited Features

- **Multiple lists per chat** – Each chat must have exactly one active list. Do not implement commands or database changes for managing multiple named lists.
- **Reminder notifications** – This bot should not send periodic reminders about outstanding items. Avoid background tasks or additional tables for this purpose.
