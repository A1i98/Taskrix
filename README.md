# Taskrix

A lightweight Rust Matrix bot that turns messages into Obsidian tasks (Markdown checkboxes) and can send daily digests.

## Features

- Creates tasks from Matrix messages and appends them to your Obsidian vault (Inbox or daily journals).
- Parses: `@+3`, `@2026-11-20`, `due::2026-11-20`, `⏳ YYYY-MM-DD` / `scheduled::...`, priorities (`! !! !!!`, `priority::high`), recurrence (`#daily/#weekly/#monthly`, `repeat::...`).
- Commands (Matrix): `!help`, `!ping`, `!version`, `!today`, `!list` (key tasks), `!list all`, `!list today`, `!list overdue`, `!done <text|number>`, `!roomid`, `!config show` (admin).
- `!done <n>` completes the nth item from the last list shown in that room.
- Auto-loads `.env` from the current working directory if present (via `dotenvy`).
- Optional daily digest to a configured room; scans only `INCLUDE_DIRS` if set, otherwise the whole vault minus `IGNORE_DIRS`.
- GitHub Actions: CI + release builds (including Linux arm64).

## Build

```bash
cargo build --release
```

## Environment variables

- `MATRIX_HS` (required): homeserver URL
- `MATRIX_USER` (required): full user ID, e.g. `@taskrix:example.com`
- `MATRIX_PASS` (required): password (token restore not enabled in this build)
- `VAULT` (required): path to Obsidian vault directory
- `INBOX_FILE` (optional): default `Inbox.md`
- `TASK_DEST` (optional): `inbox` (default) or `journal`
- `JOURNAL_DIR` (optional): default `journals`
- `JOURNAL_FORMAT` (optional): reserved for future use (default `%Y-%m-%d.md`)
- `TASKS_HEADING` (optional): heading in new journal files, e.g. `## Tasks`
- `INCLUDE_DIRS` (optional): comma-separated folders under `VAULT` to scan (if set, only these)
- `IGNORE_DIRS` (optional): comma-separated folder names to ignore (default: `.obsidian,.trash,.git,templates`)
- `STORE` (optional): SQLite store path (default `./taskrix-store`)
- `REMIND_ROOM` (optional): room ID for daily digest
- `DIGEST_AT` (optional): `HH:MM` (default `08:00`)
- `ALLOWED` (optional): comma-separated sender MXIDs (default: only `MATRIX_USER`)
- `ADMIN_USER` (optional): admin MXID for bot admin commands (default: `MATRIX_USER`)
- `LANG` (optional): `en` (default) or `fa`
- `RUST_LOG` (optional): e.g. `info`

## Run

Place a `.env` in the project directory (see `.env.example`) or export variables, then:

```bash
cargo run --release
# or
./target/release/taskrix
```

## Run example (env exports)

```bash
export MATRIX_HS=https://matrix.example.com
export MATRIX_USER=@taskrix:example.com
export MATRIX_PASS='...'
export VAULT=/path/to/obsidian/vault
export TASK_DEST=journal
export JOURNAL_DIR=daily
export TASKS_HEADING="## Tasks"
export RUST_LOG=info
./target/release/taskrix
```
