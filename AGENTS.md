# Instructions for Codex agents

- Always run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all --no-fail-fast` before committing.
- Use clear commit messages that describe the change.
- Avoid comments for obvious code. Only add comments where they improve
  understanding of non-trivial logic.
- Ensure new functionality is observable via logging. Emit at least debug logs
  for important operations so behavior can be traced in production.
- Always write tests for added functionality, preferring property-based tests
  (proptests) when feasible.
- Unit tests belong in `src/` modules within `#[cfg(test)]` blocks.
- The `tests/` directory is reserved for integration tests that rely on the
  public crate API.
- Avoid global state. Persist temporary data such as deletion selections and any notice messages in the database.
- Keep the `migrations/` directory up to date whenever the database schema changes so that embedded migrations remain in sync.
- Update `CHANGELOG.md` only for user-visible changes. Internal CI and tooling updates should not be listed. Keep pending changes under a `## Unreleased` section as a numbered list. Only create a new `## [version] - <date>` heading when a release commit is prepared. After adding the version heading, continue the next list under `Unreleased`.
- Release commits may update the manifest version to match the release's semver. After the release PR is merged, a maintainer tags that commit. The next PR should then bump the version to the next patch level and start a fresh `## Unreleased` section in `CHANGELOG.md`. The manifest version may show an unreleased patch (e.g., `0.3.1`), but the changes remain under `Unreleased` until the release commit is prepared.
- When updating versions in manifests, increment the patch version.
- This is a generic list bot. Avoid hardcoding references to "groceries" in prompts or logs. Use generic "items" wording instead.
- The bot's historic name is "Vibe Grocery Bot", but documentation and code should still refer to items generically.
- Follow the modern Rust module layout: define a `name.rs` file alongside a
  `name/` directory for submodules instead of using `mod.rs`.

# Prohibited Features

- **Multiple lists per chat** – Each chat must have exactly one active list. Do not implement commands or database changes for managing multiple named lists.
- **Reminder notifications** – This bot should not send periodic reminders about outstanding items. Avoid background tasks or additional tables for this purpose.
