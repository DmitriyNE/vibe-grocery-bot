# Vibe Grocery Bot

**WARNING: THIS ENTIRE PROJECT WAS VIBECODED WITH ONLY HOMEOPATHIC HUMAN EDITS!**

Vibe Grocery Bot is a small Telegram bot for managing a shared list of items. Each chat—whether a group or a private conversation—gets its own independent list.

## Usage

Send any message to the bot. Every non-empty line becomes an item. If you send a voice or photo message and an OpenAI API key is configured, the bot will try to recognize items automatically. A voice command like "delete milk and bread" removes those entries and the bot confirms with a list starting with a trashcan emoji. You can copy that message back to undo the deletion. The bot responds with a list message containing checkbox buttons so you can mark things bought. The main commands are:

- `/list` – show the list again
- `/archive` – archive the current list and start a new one
- `/delete` – select items to remove
- `/share` – send the list as plain text
- `/nuke` – wipe the list completely
- `/parse` – let GPT parse this message into items
- `/info` – show commit hash and whether the build is on a release or how far it is ahead of the latest release
- `/ai_mode` – run the webcam detection loop

The AI mode expects a YOLOv8 model in ONNX format. Set `YOLO_MODEL_PATH` to the
model file location or place `yolov8n.onnx` in the working directory.

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

Copy `.env.example` to `.env` and fill in the secret values. Upload them on Fly.io with:

```bash
fly secrets import < .env
```

Non-secret settings can be customised via environment variables. `config.env.example` lists the available options.

Set these variables as needed before running:

- `TELOXIDE_TOKEN` – Telegram bot token from @BotFather (secret)
- `DB_URL` – optional SQLite connection string (defaults to `sqlite:items.db`)
- `DB_POOL_SIZE` – optional maximum number of SQLite connections (defaults to `5`)
- `DELETE_AFTER_TIMEOUT` – optional delay in seconds before temporary messages are deleted (defaults to `5`)
- `RUST_LOG` – optional logging level (e.g. `info` or `debug`)
- `OPENAI_API_KEY` – optional API key for enabling voice and photo recognition (secret)
- `OPENAI_STT_MODEL` – optional model name (`whisper-1`, `gpt-4o-mini-transcribe`, or `gpt-4o-transcribe`)
- `OPENAI_GPT_MODEL` – optional chat model name (defaults to `gpt-4.1`)
- `OPENAI_VISION_MODEL` – optional vision model name (defaults to `gpt-4o`)
- `OPENAI_CHAT_URL` – optional URL for the chat completion API
- `OPENAI_STT_URL` – optional URL for the transcription API
- `YOLO_MODEL_PATH` – optional path to a YOLOv8 ONNX file used for `/ai_mode`

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
cargo nextest run --all
```

Have fun and vibe responsibly!
