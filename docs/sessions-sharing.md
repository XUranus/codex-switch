# Session sharing

`sync_sessions()` in `src/account.rs` merges all account sessions into a shared pool so that switching accounts preserves conversation history.

## Problem

Each Codex account stores conversation sessions as JSONL files under `<codex-home>/sessions/<YYYY>/<MM>/<session-id>.jsonl`. With `codex-switch`, each account has its own directory — and therefore its own `sessions/`. When you switch from `default` to `personal`, the sessions from `default` become invisible because `~/.codex` now points to `~/.codex-personal/`, which has a different (or empty) `sessions/` directory.

## Solution: shared sessions pool

A single directory `~/.codex-sessions/` holds all sessions. Each account's `sessions/` is replaced with a symlink to this pool.

```
~/.codex-sessions/
  2026/04/21/rollout-....jsonl    ← from default
  2026/05/01/rollout-....jsonl    ← from personal
  2026/05/02/rollout-....jsonl    ← merged (larger version kept)
  ...

~/.codex-default/sessions   →  symlink → ~/.codex-sessions/
~/.codex-personal/sessions  →  symlink → ~/.codex-sessions/
```

When Codex writes to `~/.codex/sessions/...`, the kernel resolves:
`~/.codex` → `~/.codex-personal/` → `sessions/` → `~/.codex-sessions/`

All accounts read and write the same pool. Switching accounts no longer hides sessions.

## The sync command

```
codex-switch sync [extra-paths...]
```

### Phase 1: Collect sources

`sync` gathers session directories from:

1. **Each managed account** — `~/.codex-<alias>/sessions` if it exists as a real directory (not already a symlink).
2. **Extra paths** — each positional argument after `sync` is treated as a sessions directory to merge from.

A source is skipped if its `sessions/` is already a symlink (already linked to the pool from a previous sync).

### Phase 2: Merge into pool

For each source, `merge_into_pool()` recursively walks the directory and copies files into `~/.codex-sessions/`, preserving the relative path (e.g. `2026/04/21/rollout-....jsonl`).

**Deduplication rule**: when a file already exists in the pool (same relative path = same session ID):

| Condition | Action | Counter |
|-----------|--------|---------|
| Source file > pool file | Copy source over pool | `merged++` |
| Source file ≤ pool file | Keep pool version | `skipped++` |
| File not in pool | Copy to pool | `added++` |

This means the **largest version of each session survives**. A session that was resumed across multiple accounts keeps the most complete copy.

### Phase 3: Symlink accounts

After merging, `replace_with_symlink()` runs for each account:

1. If `sessions/` is already a symlink to the pool → skip (idempotent).
2. If `sessions/` is a symlink to somewhere else → remove the old symlink.
3. If `sessions/` is a real directory → remove it entirely (contents already merged).
4. Create symlink: `sessions → ~/.codex-sessions`.

Also, any extra path that looks like a Codex home (has `auth.json`) gets the same symlink treatment.

### Output

```
$ codex-switch sync ~/old-codex/sessions
Merging sessions into shared pool...
  default: +23 files, ~1 skipped, ~0 merged (kept larger)
  personal: +1 files, ~0 skipped, ~0 merged (kept larger)
  sessions: +2 files, ~0 skipped, ~0 merged (kept larger)
  2 account(s) symlinked → /home/xuranus/.codex-sessions
Done: 26 added, 1 skipped, 0 merged (kept larger).
```

## Import and the sessions pool

When importing a new account with `codex-switch import`:

- If the shared pool **already exists**: the new account's `sessions/` is immediately symlinked to the pool (no copying needed). The account inherits all existing sessions.
- If the pool **doesn't exist yet**: the source directory's `sessions/` is copied verbatim into the new account, preserving whatever sessions the source had. Running `sync` later will merge them into the pool.

## Idempotency

Running `sync` multiple times is safe:

- Accounts whose `sessions/` is already a symlink to the pool are skipped (no re-copying).
- Files already in the pool are deduplicated by size (only overwritten if the source has a larger version).
- The pool directory itself is never removed — only added to.

## Cross-platform notes

Symlinks work on Linux and macOS natively. On Windows, directory symlinks require either administrator privileges or Developer Mode to be enabled. If symlink creation fails on Windows (permission error), the tool reports the error but continues — the sessions files are still merged into the pool; only the symlink step is skipped.
