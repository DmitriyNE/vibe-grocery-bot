# Vibe Grocery Bot

**WARNING: THIS ENTIRE PROJECT WAS VIBECODED WITH ONLY HOMEOPATHIC HUMAN EDITS!**

This is a small Telegram bot for keeping a collaborative grocery list. The code came into being through vibes and automated tooling rather than meticulous engineering, so expect quirks.

## Development

You'll need a recent [Rust toolchain](https://www.rust-lang.org/tools/install). The CI configuration uses `cargo fmt`, `cargo clippy` and `cargo test` so running them locally is a good idea too.

All project dependencies, including `sqlx` and `teloxide`, are fetched automatically by Cargo when building.

The bot now manages its database schema through embedded SQLx migrations. When

the application starts it will automatically run any migrations found in the
`migrations/` directory.

## Configuration

The bot reads its settings from environment variables:

* `TELOXIDE_TOKEN` - your Telegram bot token.
* `DB_URL` - SQLite connection string (defaults to `sqlite:shopping.db`).
  The application ensures the database file is created if it does not exist.

Have fun and vibe responsibly!

## Installation

1. Install a recent [Rust toolchain](https://www.rust-lang.org/tools/install).
2. Clone this repository and build the binary:

   ```bash
   cargo build --release
   ```

   You can also build a container image using the provided `Dockerfile`:

   ```bash
   docker build -t shopbot .
   ```

## Usage

Set the following environment variables before running the bot:

| Variable         | Description                                  |
| ---------------- | -------------------------------------------- |
| `TELOXIDE_TOKEN` | Telegram bot token obtained from @BotFather. |
| `DB_URL`         | (Optional) Database connection string.       |

If `DB_URL` is not provided the bot defaults to a local SQLite
database in the current directory.

Launch the bot with:

```bash
cargo run --release
```

On first start it will create the database (if needed) and run any
migrations found in `migrations/` automatically.

## Using the Bot

Send any message to the bot and each line becomes a shopping list item.
The bot replies with an interactive message where every item has its own
checkbox button. Tap a button to mark something as bought or to uncheck it.

Available commands:

- `/list` &mdash; show the current list again
- `/archive` &mdash; finalize and archive the list, starting a new one
- `/delete` &mdash; open a panel to select and remove items
- `/nuke` &mdash; completely wipe the current list

The bot keeps one active list per chat or group. Items and their state are
stored separately for each chat, so you can use the bot in group
conversations or personal ones without interference.

## Deployment on Fly.io

The repository includes `fly.toml` so you can deploy the bot to
[Fly.io](https://fly.io/). Edit this file to set your own Fly app name
before the first deploy. After installing the Fly CLI and logging in,
run the following commands:

```bash
fly volumes create shopbot_db --size 1
fly secrets set TELOXIDE_TOKEN=YOUR_TOKEN
fly deploy
```

The volume stores the SQLite database under `/data` as configured in
`fly.toml`. Subsequent deployments will reuse the same data.
