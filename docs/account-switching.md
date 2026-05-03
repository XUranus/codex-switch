# Account switching

`switch_to()` in `src/account.rs:207-242` implements the `use` command. It repoints the `~/.codex` symlink to make a different account active.

## The three states of ~/.codex

### State 1: Real directory (first switch ever)

Before `codex-switch` is used, `~/.codex` is a regular directory created by Codex. It contains `auth.json`, config, caches, history — a full Codex home.

```
~/.codex/
  auth.json          ← the currently logged-in account
  config.toml
  cache/
  state_5.sqlite
  history.jsonl
```

On the first `use`, the directory is **renamed** to `~/.codex-default`, preserving all data:

```rust
// src/account.rs:218-229
if codex_home.exists() && codex_home.is_dir() && fs::read_link(&codex_home).is_err() {
    let default_path = home_dir().join(".codex-default");
    if !default_path.exists() {
        fs::rename(&codex_home, &default_path)?;
    }
}
```

After rename:
```
~/.codex  → (gone, was renamed)

~/.codex-default/
  auth.json          ← preserved, the "default" account
  config.toml
  cache/
  state_5.sqlite
  history.jsonl
```

### State 2: Symlink (subsequent switches)

After the first switch, `~/.codex` is always a symlink. Subsequent switches remove the old symlink and create a new one:

```rust
// src/account.rs:232-235
if codex_home.is_symlink() {
    fs::remove_file(&codex_home)?;
}
```

`remove_file` on a symlink removes the symlink itself, not the target directory. The target (e.g. `~/.codex-default`) is untouched.

### State 3: Missing (fresh system)

If `~/.codex` doesn't exist at all, the symlink is created directly:

```rust
create_dir_symlink(&target.path, &codex_home)?;
```

This could happen on a system that has never run `codex`, or if the symlink was manually deleted.

## Switch step by step

```
$ codex-switch use personal
```

1. **Discover accounts**: `discover()` scans `~/.codex-*` and `~/.codex` to build the account list.

2. **Find the target**: locate the account whose alias is `"personal"`.

   ```rust
   let target = accounts.iter()
       .find(|a| a.alias == alias)
       .ok_or_else(|| format!("no account named '{}'", alias))?;
   ```

3. **Check current state of ~/.codex**:
   - If it's a real directory → rename to `~/.codex-default`
   - If it's a symlink → remove the symlink
   - If it doesn't exist → proceed

4. **Create the symlink**:

   ```rust
   unix_fs::symlink(&target.path, &codex_home)?;
   ```

   `target.path` is the full path to the account directory, e.g. `/home/xuranus/.codex-personal`. `codex_home` is `/home/xuranus/.codex`.

5. **Return**: the target account struct is returned to the caller, which prints `"Switched to personal (xuranus42@qq.com)"`.

## What this means for Codex

After the switch, any `codex` invocation resolves `~/.codex` through the filesystem:

```
codex reads ~/.codex/auth.json
  → kernel resolves symlink ~/.codex → /home/xuranus/.codex-personal
  → open("/home/xuranus/.codex-personal/auth.json")
  → reads account B's tokens
```

Codex itself has no idea a switch happened. It just opens the file at the path it always uses. The symlink is transparent.

## Safety properties

**No data loss.** The rename on first switch moves, not copies — but the data is preserved at the new path. The account becomes available as `"default"` and can be switched back to.

**Symlink removal is safe.** `fs::remove_file` on a symlink only removes the link, never the target. `rm ~/.codex` when it's a symlink removes the 0-byte link inode, not `~/.codex-personal/`.

**Target existence not enforced.** The symlink is created even if the target directory doesn't contain a valid `auth.json`. This avoids creating a situation where `~/.codex` is a dangling symlink — it always points to a real directory (the account dir), even if that directory's contents are incomplete. Codex will handle auth errors at its own level.

**CODEX_HOME still works.** The env var takes priority over the default path. If `CODEX_HOME` is set, Codex uses it directly, bypassing `~/.codex` entirely. This means you can use an unmanaged account without switching, and existing scripts that set `CODEX_HOME` continue to work.

## Switching to the same account

If the target account is already active (symlink already points to the right directory), the symlink is removed and recreated with the same target. This is a no-op from the user's perspective — `codex` sees the same `auth.json` before and after.
