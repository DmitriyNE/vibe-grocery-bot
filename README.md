# Vibe Grocery Bot

**WARNING: THIS ENTIRE PROJECT WAS VIBECODED WITH ONLY HOMEOPATHIC HUMAN EDITS!**

Vibe Grocery Bot is a small Telegram bot for managing a shared shopping list. Each chat—whether a group or a private conversation—gets its own independent list.

## Usage

Send any message to the bot. Every non-empty line becomes an item. If you send a voice or photo message and an OpenAI API key is configured, the bot will try to recognize items automatically. The bot responds with a list message containing checkbox buttons so you can mark things bought. The main commands are:

- `/list` – show the list again
- `/archive` – archive the current list and start a new one
- `/delete` – select items to remove
- `/share` – send the list as plain text
- `/nuke` – wipe the list completely
- `/parse` – let GPT parse this message into items

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
- `RUST_LOG` – optional logging level (e.g. `info` or `debug`)
- `OPENAI_API_KEY` – optional API key for enabling voice and photo recognition
- `OPENAI_STT_MODEL` – optional model name (`whisper-1`, `gpt-4o-mini-transcribe`, or `gpt-4o-transcribe`)

The database file is created automatically if needed. Embedded SQLx migrations in the `migrations/` directory are executed on startup.

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

Before committing, run:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --no-fail-fast
```

Have fun and vibe responsibly!
