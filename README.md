# Vibe Grocery Bot

**WARNING: THIS ENTIRE PROJECT WAS VIBECODED WITH ONLY HOMEOPATHIC HUMAN EDITS!**

This is a small Telegram bot for keeping a collaborative grocery list. The code came into being through vibes and automated tooling rather than meticulous engineering, so expect quirks.

## Development

You'll need a recent Rust toolchain with `sqlx` and `teloxide` dependencies. The CI configuration uses `cargo fmt`, `cargo clippy` and `cargo test` so running them locally is a good idea too.

The bot now manages its database schema through embedded SQLx migrations. When

the application starts it will automatically run any migrations found in the
`migrations/` directory.

## Configuration

The bot reads its settings from environment variables:

* `TELOXIDE_TOKEN` - your Telegram bot token.
* `DB_URL` - SQLite connection string (defaults to `sqlite:shopping.db`).
  The application ensures the database file is created if it does not exist.

Have fun and vibe responsibly!
