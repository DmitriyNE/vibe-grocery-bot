# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2025-06-07
### Added
- Initial release of the Telegram shopping list bot.
- Each chat has a single list; send text to add items line by line.
- Inline checkbox buttons allow marking items as bought.
- `/list` shows the current list.
- `/archive` finalizes and archives the list, starting a new one.
- `/delete` opens a panel to delete items.
- `/share` sends the list as plain text.
- `/nuke` completely deletes the list.
- `/parse` uses GPT to parse a message into items.
- Optional voice message parsing via OpenAI speech-to-text.

