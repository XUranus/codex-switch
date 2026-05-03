# codex-switch

Manage multiple Codex CLI accounts without logging out and back in. Switch between accounts by repointing a symlink — no environment variables needed.

## Why

Codex CLI reads `~/.codex/auth.json` to know who you are. If you have multiple accounts (e.g. work and personal, or multiple accounts to work around token quotas), you either re-login each time or set `CODEX_HOME` to point at different directories.

`codex-switch` keeps each account in its own `~/.codex-<alias>` directory and makes `~/.codex` a symlink to the active one. Switching accounts is a single command.

## Installation

```bash
cargo install --path .
```

This places `codex-switch` in `~/.cargo/bin/` — make sure that's on your `PATH`.

## Guide: logging into multiple accounts

Codex CLI only stores one login at a time under `~/.codex`. To set up a second account, temporarily redirect `CODEX_HOME` while logging in:

### Step 1: Log into your first account (already done)

If you already use Codex, your first account is at `~/.codex`. This is your "default" account — codex-switch will preserve it.

```
$ codex login          # normal login, stores tokens in ~/.codex/auth.json
```

### Step 2: Log into a second account

Use `codex-switch login <name>` — a wrapper around `codex login` that stores tokens in `~/.codex-<name>`:

```bash
$ codex-switch login second
Starting Codex login for 'second'...
A browser window will open — authenticate with your other account.

Logged in as bob@work.com (second) — use `codex-switch use second` to activate.
```

This runs the standard Codex login flow — open the URL in a browser, authenticate — but stores the tokens in `~/.codex-second/` instead of `~/.codex`. Your first account's tokens remain untouched.

Repeat for each additional account:

```bash
codex-switch login work
codex-switch login client-a
```

### Step 3: List accounts

```bash
$ codex-switch list
→ default          alice@personal.com              9cf65c60
  second           bob@work.com                    b061a30a
  work             carol@example.com               c172b41b
```

The arrow shows which account is currently active (the one `codex` will use).

### Step 4: Switch between accounts

```bash
$ codex-switch use second
Switched to second (bob@work.com)

$ codex whoami      # now runs as bob@work.com
```

Switch back anytime:

```bash
$ codex-switch use default
Switched to default (alice@personal.com)
```

### Step 5: Share sessions across accounts

After switching a few times, you'll notice sessions from one account aren't visible from another. Run `sync` once to merge all sessions into a shared pool:

```bash
$ codex-switch sync
Merging sessions into shared pool...
  default: +23 files, ~0 skipped, ~0 merged (kept larger)
  second: +5 files, ~0 skipped, ~0 merged (kept larger)
  2 account(s) symlinked → /home/you/.codex-sessions
Done: 28 added, 0 skipped, 0 merged (kept larger).
```

After this, all accounts see each other's conversations. Each account's `sessions/` directory is now a symlink to `~/.codex-sessions/` — new sessions from any account land in the shared pool automatically.

### Workflow summary

```bash
# One-time setup per new account
codex-switch login <name>  # authenticate a new account

# One-time sessions merge
codex-switch sync

# Day-to-day usage
codex-switch list          # see all accounts
codex-switch use <name>    # switch to another account
codex-switch current       # confirm which account is active
```

## Usage

```bash
codex-switch list                 # List all accounts
codex-switch current              # Show the active account
codex-switch use <name>           # Switch to a specific account
codex-switch import <name> <path> # Import an existing CODEX_HOME directory
codex-switch sync [paths...]     # Merge sessions into shared pool
```

## Commands

### `list`

Scans your home directory for `~/.codex-*` directories and the current `~/.codex`. Shows the alias, email, and account ID for each. An arrow (`→`) marks the currently active account.

```
→ default          xuranus@protonmail.com         9cf65c60
  personal         xuranus42@qq.com               b061a30a
```

### `current`

Prints the email and alias of the active account.

```
xuranus42@qq.com (personal)
```

### `use <name>`

Switches the `~/.codex` symlink to point to `~/.codex-<name>`. On the very first switch, if `~/.codex` is still a regular directory, it gets renamed to `~/.codex-default` so no data is lost.

```
$ codex-switch use personal
Switched to personal (xuranus42@qq.com)
```

### `login <name>`

Runs `codex login` with `CODEX_HOME` pointed at `~/.codex-<name>`. Creates the directory if needed and opens a browser for authentication. This is the simplest way to add a new account.

```
$ codex-switch login work
Starting Codex login for 'work'...
A browser window will open — authenticate with your other account.

Logged in as work@example.com (work) — use `codex-switch use work` to activate.
```

### `import <name> <path>`

Imports an existing CODEX_HOME directory (e.g. one you previously used via `CODEX_HOME=~/some/path codex`) as a named account. Copies only identity files — auth tokens, config, rules, skills, memories — not caches or logs.

```
$ codex-switch import work ~/backups/.codex-work-v1
Imported 'work' (work@example.com). Use `codex-switch use work` to activate.
```

### `sync [paths...]`

Merges session files from all accounts into a shared `~/.codex-sessions/` pool, then replaces each account's `sessions/` directory with a symlink to the pool. For sessions with the same filename (same session ID), the larger file is kept.

Optional extra paths can be passed to merge sessions from directories outside the managed set.

```
$ codex-switch sync ~/old-codex-backup/sessions
Merging sessions into shared pool...
  default: +23 files, ~0 merged (kept larger)
  personal: +0 files, ~1 merged (kept larger)
  sessions: +2 files, ~0 merged (kept larger)
Done: 25 session files added, 1 merged (kept larger).
```

After syncing, all accounts see the same sessions — switching accounts with `use` no longer hides your conversation history.

## How it works

Each account lives in a directory named `~/.codex-<alias>`. `~/.codex` is a symlink pointing to whichever account is active. Codex reads `~/.codex` by default, so it always sees the active account's `auth.json`.

```
~/.codex          →  symlink → ~/.codex-personal
~/.codex-default/     auth.json, config.toml, ...
~/.codex-personal/    auth.json, config.toml, ...
```

Switching accounts is `rm ~/.codex && ln -s ~/.codex-<target> ~/.codex`. `CODEX_HOME`, if set, overrides the symlink — so existing scripts still work.

Sessions are stored in a shared `~/.codex-sessions/` pool. Each account's `sessions/` is a symlink to this pool, so all accounts see the same conversation history. `sync` performs the one-time migration from per-account sessions into the shared pool.

See [docs/how-it-works.md](docs/how-it-works.md) for the full details.

## Build

```bash
cargo build              # debug
cargo build --release    # optimized (~680K binary)
```

## Dependencies

Only `serde` and `serde_json`. No other third-party crates.
