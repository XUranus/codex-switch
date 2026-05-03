# Account discovery

`discover()` in `src/account.rs:128-199` is responsible for finding all managed Codex accounts on the system. It determines **what** accounts exist, **where** they live, and **which one** is currently active.

## Scanning `~/.codex-*` directories

The primary source of accounts is `~/.codex-<alias>` directories. These are created by `import` or by the first `use` call (which renames the original `~/.codex` to `~/.codex-default`).

```rust
// src/account.rs:137-168
let home = home_dir();
if let Ok(entries) = fs::read_dir(&home) {
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with(".codex-") {
            continue;  // skip anything not matching the pattern
        }
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;  // skip files, only interested in directories
        }
        let alias = name_str.strip_prefix(".codex-").unwrap();
        // ... read auth.json and build Account struct
    }
}
```

For each `~/.codex-*` directory, it:

1. Extracts the **alias** — the part after `.codex-` (e.g. `personal` from `.codex-personal`).
2. Reads `<path>/auth.json` to get the **email** and **account_id**.
3. Determines if this account is **active** by comparing the directory's path to the `~/.codex` symlink target.

## The "default" account

Before `codex-switch` is ever used, `~/.codex` is a real directory — the standard Codex home. `discover()` handles this case:

```rust
// src/account.rs:170-183
if !codex_is_symlink && codex_home.is_dir() {
    let auth_path = codex_home.join("auth.json");
    if auth_path.exists() {
        // Treat as an account with alias "default"
        accounts.push(Account {
            alias: "default".into(),
            ...
            path: codex_home.clone(),
            active: true,
        });
    }
}
```

This "default" account is treated exactly like any `~/.codex-*` account. On the first `use`, it gets renamed to `~/.codex-default`, making it a proper managed account with a stable alias.

## Determining the active account

There are two paths for determining which account is active:

### 1. `~/.codex` is a symlink

```rust
let active_target = fs::read_link(&codex_home).ok();
// ...
let active = if let Some(ref target) = active_target {
    target == &path || fs::canonicalize(&codex_home).ok() == fs::canonicalize(&path).ok()
} else {
    false
};
```

The symlink target (e.g. `/home/xuranus/.codex-personal`) is compared to each account's path. The `canonicalize` call handles relative vs. absolute symlink targets.

### 2. `~/.codex` is a real directory

If `~/.codex` is not a symlink but exists as a directory (pre-tool state), it IS the active account — the implicit "default". It gets `active: true` automatically.

### 3. Edge case: symlink to an external path

If `~/.codex` is a symlink to a path that doesn't match `~/.codex-*` (e.g. a manually created symlink to `/some/other/dir`), a second pass resolves the symlink with `canonicalize` and matches against all known account paths:

```rust
// src/account.rs:185-195
if codex_is_symlink {
    if let Ok(resolved) = fs::canonicalize(&codex_home) {
        for acc in &mut accounts {
            if fs::canonicalize(&acc.path).ok() == Some(resolved.clone()) {
                acc.active = true;
            }
        }
    }
}
```

## Account identity: alias vs. account_id

Two accounts can share the same `account_id` (same login) in different directories with different aliases. `discover()` doesn't deduplicate — it lists both. The **alias** is the filesystem name (user-chosen, mutable via rename), while the **account_id** is the opaque UUID from Codex (immutable, stored in `auth.json`).

## Result ordering

Results are sorted alphabetically by alias, with the active account marked by an arrow (`→`) in `list` output. The sort is stable and predictable, making `list` output easy to scan regardless of directory creation order.
