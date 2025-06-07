# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - Unreleased
1. Add items by sending a photo using OpenAI vision to detect items automatically.
2. Display a check mark when every item in the list is checked off.

## [0.1.0] - 2025-06-07
1. Initial release of the Telegram shopping list bot.
2. Each chat has a single list; send text to add items line by line.
3. Inline checkbox buttons allow marking items as bought.
4. `/list` shows the current list.
5. `/archive` finalizes and archives the list, starting a new one.
6. `/delete` opens a panel to delete items.
7. `/share` sends the list as plain text.
8. `/nuke` completely deletes the list.
9. `/parse` uses GPT to parse a message into items.
10. Optional voice message parsing via OpenAI speech-to-text.
11. Configurable speech model via `OPENAI_STT_MODEL` in `fly.toml`.

