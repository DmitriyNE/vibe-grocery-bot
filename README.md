# Vibe Grocery Bot

Vibe Grocery Bot is a small Telegram bot for managing a shared shopping list. It was vibecoded mostly automatically, so treat it as a fun hack.

Each chat—whether a group or a private conversation—gets its own independent list.

## Usage

Send any message to the bot. Every non-empty line becomes an item. The bot responds with a list message containing checkbox buttons so you can mark things bought. The main commands are:

- `/list` – show the list again
- `/archive` – archive the current list and start a new one
- `/delete` – select items to remove
- `/nuke` – wipe the list completely

## Installation

1. Install a recent [Rust toolchain](https://www.rust-lang.org/tools/install).
2. Clone this repository and build the binary:

   ```bash
   cargo build --release
   ```

   Or build a container image:

   ```bash
   docker build -t shopbot .
   ```

## Configuration

Set these environment variables before running:

- `TELOXIDE_TOKEN` – Telegram bot token from @BotFather
- `DB_URL` – optional SQLite connection string (defaults to `sqlite:shopping.db`)

The database file is created automatically if needed. Any migrations in `migrations/` run on startup.

## Running

Launch the bot with:

```bash
cargo run --release
```

## Deployment on Fly.io

The repository includes `fly.toml` for deployment on [Fly.io](https://fly.io/). Edit the `app` name inside that file, then after installing the Fly CLI run:

```bash
fly volumes create shopbot_db --size 1
fly secrets set TELOXIDE_TOKEN=YOUR_TOKEN
fly deploy
```

The volume stores the SQLite database under `/data`.

## Development

Dependencies like `teloxide` and `sqlx` are fetched by Cargo. Before committing, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --no-fail-fast
```

Have fun and vibe responsibly!
