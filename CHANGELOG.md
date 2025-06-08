# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## [0.3.0] - 2025-06-09
1. `/info` command shows commit hash and release version or how many commits the build is ahead of the latest release
2. Remove items via voice commands like "delete milk". The bot replies with a copyable list of the deleted entries.
3. Voice requests prefer nominative item names and numeric quantities when interpreting additions.
4. Voice command prompt now ensures deletions return items exactly as listed so numbers aren't dropped.
5. Voice GPT prompt includes the list in JSON form to reduce misunderstandings.
6. Empty voice transcriptions are ignored instead of confusing the GPT command
   parser.
7. GPT chat model is configurable via `OPENAI_GPT_MODEL` and now defaults to
   `gpt-4.1`.

## [0.2.0] - 2025-06-08
1. Add items by sending a photo using OpenAI vision to detect items automatically.
2. Items show green checkmarks when the entire list is checked off.
3. Checkbox icons indicate item status in the list.
4. Deletion mode highlights selections with red crosses, shows empty squares for
   unselected items, and uses a trashcan icon to finish.
5. Reduced noisy HTTP logs by setting the `hyper` log level to info in `fly.toml`.
6. Voice transcription uses an improved prompt so numbers like "1 milk" are preserved.
7. GPT item extraction keeps numbers intact, preventing "1 milk" becoming "milk".


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

