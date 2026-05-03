# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`codex-switch` is a CLI tool to manage multiple Codex CLI accounts. Each account lives in its own `~/.codex-<alias>` directory. `~/.codex` is a symlink pointing to the active account. Codex reads `~/.codex` by default, so switching accounts is a matter of repointing the symlink — no env vars needed.

## Build & Run

```
cargo build              # debug build
cargo build --release    # release build (~678K binary)
cargo run -- <args>      # run with args
```

## Architecture

- `src/main.rs` — CLI entrypoint: parses subcommands (`list`, `current`, `use`, `import`, `sync`) and dispatches
- `src/account.rs` — core logic: account discovery (scans `~/.codex-*` dirs), auth.json parsing (email extraction from JWT id_token via base64 decode), symlink management (switch creates/updates `~/.codex` symlink), filtered import (copies only identity files, skipping heavy caches/logs), shared sessions pool (`~/.codex-sessions/`) with merge-dedup (keep larger file for same session ID)

Dependencies: `serde` + `serde_json` for auth.json parsing. No other third-party crates.

## How account switching works

1. Account dirs are `~/.codex-<alias>` (e.g. `~/.codex-personal`, `~/.codex-work`)
2. `~/.codex` is a symlink → active account dir
3. On first `use`, if `~/.codex` is a real directory, it gets renamed to `~/.codex-default`
4. `import` copies only identity files (auth.json, config.toml, etc.), not caches or logs
5. `CODEX_HOME` env var, if set, overrides everything (backward compatible)
6. `sync` merges all account sessions into `~/.codex-sessions/`, then each account's `sessions/` becomes a symlink to the shared pool — switching accounts preserves conversation history
