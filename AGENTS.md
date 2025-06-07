# Instructions for Codex agents

- Always run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all --no-fail-fast` before committing.
- Use clear commit messages that describe the change.
- Avoid comments for obvious code. Only add comments where they improve
  understanding of non-trivial logic.
- Ensure new functionality is observable via logging. Emit at least debug logs
  for important operations so behavior can be traced in production.
- Always write tests for added functionality, preferring property-based tests
  (proptests) when feasible.
- Avoid global state. Persist temporary data such as deletion selections and any notice messages in the database.
- Keep the `migrations/` directory up to date whenever the database schema changes so that embedded migrations remain in sync.
- Update `CHANGELOG.md` with a new entry for any user-visible change. Keep pending changes at the top as a numbered list. When the project version is bumped, insert a `## [version] - <date>` header below that list and start a new numbered list for the next release.

# Prohibited Features

- **Multiple lists per chat** – Each chat must have exactly one active list. Do not implement commands or database changes for managing multiple named lists.
- **Reminder notifications** – This bot should not send periodic reminders about outstanding items. Avoid background tasks or additional tables for this purpose.
