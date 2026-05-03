# How codex-switch works

## The core idea: a symlink as a switcher

Codex reads its configuration from the directory pointed to by the `CODEX_HOME` environment variable, or `~/.codex` if the variable is unset. Inside that directory, `auth.json` contains the login tokens that identify the current user.

To support multiple accounts, we give each one its own directory — `~/.codex-<alias>` — and make `~/.codex` a **symlink** to the one we want active.

```
~/.codex           →  symlink → ~/.codex-personal
~/.codex-default/      auth.json  (account A tokens)
~/.codex-personal/     auth.json  (account B tokens)
```

When `codex` runs, the kernel resolves `~/.codex/auth.json` through the symlink to `~/.codex-personal/auth.json`. Neither Codex nor any other tool needs to be aware of the indirection.

This approach has several properties:

- **Persistent across shells.** Unlike `export CODEX_HOME=...`, a symlink survives terminal sessions.
- **No eval or shell wrappers needed.** The tool is a plain binary that manipulates the filesystem.
- **Backward compatible.** If `CODEX_HOME` is set, Codex ignores `~/.codex` and uses the env var directly. Set `CODEX_HOME` to bypass the symlink when needed.

## Architecture overview

```
main.rs              CLI dispatch (list, current, use, import, sync)
  │
  └─ account.rs      All core logic
       ├─ discover()           Scan filesystem for accounts
       ├─ current()            Find which account is active
       ├─ switch_to()          Repoint the ~/.codex symlink
       ├─ import_account()     Copy an external CODEX_HOME into ~/.codex-<name>
       ├─ sync_sessions()      Merge sessions → shared pool, symlink accounts
       ├─ read_email_and_id()  Parse auth.json → email + account_id
       ├─ decode_jwt_payload() Base64url-decode a JWT to extract claims
       └─ copy_dir_filtered()  Copy only identity files, skip caches
```

## The three main operations in detail

### 1. Account discovery

See [account-discovery.md](account-discovery.md).

`discover()` scans `$HOME` for two kinds of directories:

- **Named accounts**: directories matching `~/.codex-*` (e.g. `~/.codex-personal`, `~/.codex-work`). Each contains a full Codex home directory with its own `auth.json`.
- **The default account**: if `~/.codex` exists as a real directory (not a symlink), it's treated as an implicit account with the alias `"default"`. This captures the pre-tool state — whatever account you were logged into before installing codex-switch.

For each directory found, it reads `auth.json` to extract the email and account ID. The active account is determined by resolving the `~/.codex` symlink (if any) and comparing the resolved path to each account's path.

### 2. Auth extraction

See [auth-extraction.md](auth-extraction.md).

Codex stores authentication in `auth.json` as JSON. The tokens include a JWT `id_token` (OpenID Connect). Email can be present in two places:

1. **`tokens.email`** — a direct field, present in some Codex versions.
2. **Inside the `id_token` JWT** — a signed JWT whose payload (decoded without verification) contains `email`, `account_id`, and other claims.

`read_email_and_id()` tries both. For the JWT path, `decode_jwt_payload()` splits the token on `.`, converts the middle segment from base64url to standard base64, decodes it, and parses the resulting JSON. No cryptographic verification is performed — the signature is ignored, since we only need identity metadata, not proof of authentication.

### 3. Account switching

See [account-switching.md](account-switching.md).

`switch_to()` handles three states of `~/.codex`:

| State | Action |
|-------|--------|
| Real directory (first switch) | Rename to `~/.codex-default`, then create symlink |
| Symlink (subsequent switches) | Remove old symlink, create new one |
| Missing | Create symlink directly |

The rename on first switch preserves all data — caches, history, config, everything. The old account becomes available as `"default"` and can be switched back to at any time.

### Import

`import_account()` brings an external CODEX_HOME directory into the managed set. Instead of copying everything (which could be hundreds of megabytes of logs and caches), it copies only identity-defining files:

```
auth.json, config.toml, version.json, installation_id,
.personality_migration, rules/, skills/, memories/
```

Caches, SQLite databases, history files, and generated images are skipped — Codex regenerates these on first run.

### 4. Session sharing

See [sessions-sharing.md](sessions-sharing.md).

Codex stores conversation sessions as JSONL files under `<codex-home>/sessions/<YYYY>/<MM>/`. Without intervention, each account has its own sessions directory — switching accounts hides previous conversations.

`sync_sessions()` solves this with a **shared sessions pool**:

1. Creates `~/.codex-sessions/` as a single shared directory.
2. Walks every known account's `sessions/` directory and copies files into the pool.
3. For files with the **same relative path** (same session ID): compares sizes, keeps the **larger** file. A `+merged` counter tracks how many files were replaced with larger versions; a `+skipped` counter tracks how many were already larger or equal in the pool.
4. Replaces each account's `sessions/` directory with a **symlink → `~/.codex-sessions/`**.

Afterward, any account writes its sessions through the symlink into the shared pool, and reads see the unified history. Extra paths can be passed to `sync` to merge sessions from directories outside the managed set.

## Filesystem layout after first use

```
$HOME/
  .codex            → symlink → .codex-personal   (resolved by codex at runtime)
  .codex-sessions/                                 (shared sessions pool)
    2026/05/02/rollout-....jsonl
    2026/05/01/rollout-....jsonl
    ...
  .codex-default/                                  (original account, preserved)
    auth.json
    config.toml
    cache/
    sessions  → symlink → ../.codex-sessions
    ...
  .codex-personal/                                 (imported or previously active)
    auth.json
    config.toml
    sessions  → symlink → ../.codex-sessions
    ...
```

## CODEX_HOME override

If `CODEX_HOME` is set in the environment, Codex uses that path directly, bypassing `~/.codex` entirely. This means:

- Scripts that set `CODEX_HOME` continue to work unchanged.
- You can temporarily use an account without switching: `CODEX_HOME=~/.codex-other codex ...`
- `codex-switch` itself is unaware of `CODEX_HOME` — it only manages the `~/.codex` symlink.
