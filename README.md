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

## Usage

```bash
codex-switch list                 # List all accounts
codex-switch current              # Show the active account
codex-switch use <name>           # Switch to a specific account
codex-switch import <name> <path> # Import an existing CODEX_HOME directory
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

### `import <name> <path>`

Imports an existing CODEX_HOME directory (e.g. one you previously used via `CODEX_HOME=~/some/path codex`) as a named account. Copies only identity files — auth tokens, config, rules, skills, memories — not caches or logs.

```
$ codex-switch import work ~/backups/.codex-work-v1
Imported 'work' (work@example.com). Use `codex-switch use work` to activate.
```

## How it works

Each account lives in a directory named `~/.codex-<alias>`. `~/.codex` is a symlink pointing to whichever account is active. Codex reads `~/.codex` by default, so it always sees the active account's `auth.json`.

```
~/.codex          →  symlink → ~/.codex-personal
~/.codex-default/     auth.json, config.toml, ...
~/.codex-personal/    auth.json, config.toml, ...
```

Switching accounts is `rm ~/.codex && ln -s ~/.codex-<target> ~/.codex`. `CODEX_HOME`, if set, overrides the symlink — so existing scripts still work.

See [docs/how-it-works.md](docs/how-it-works.md) for the full details.

## Build

```bash
cargo build              # debug
cargo build --release    # optimized (~680K binary)
```

## Dependencies

Only `serde` and `serde_json`. No other third-party crates.
