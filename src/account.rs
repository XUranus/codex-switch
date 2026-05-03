use serde::Deserialize;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

const CODEX_DIR: &str = ".codex";

#[derive(Debug, Clone)]
pub struct Account {
    pub alias: String,
    pub email: String,
    pub account_id: String,
    /// Path to the actual directory (e.g. ~/.codex-personal, ~/some/other/codex)
    pub path: PathBuf,
    /// Whether this account is the currently active one
    pub active: bool,
}

/// Minimal struct to extract what we need from auth.json
#[derive(Debug, Deserialize)]
struct AuthJson {
    tokens: Option<AuthTokens>,
}

#[derive(Debug, Deserialize)]
struct AuthTokens {
    #[serde(rename = "account_id")]
    account_id: Option<String>,
    id_token: Option<String>,
    email: Option<String>,
}

/// Get the email from auth.json.
/// Priority: tokens.email (direct) → decode JWT id_token → "unknown"
fn read_email_and_id(auth_path: &Path) -> (String, String) {
    let data = match fs::read_to_string(auth_path) {
        Ok(s) => s,
        Err(_) => return ("(no auth.json)".into(), "unknown".into()),
    };

    let auth: AuthJson = match serde_json::from_str(&data) {
        Ok(a) => a,
        Err(_) => return ("(invalid json)".into(), "unknown".into()),
    };

    let tokens = match auth.tokens {
        Some(t) => t,
        None => return ("(no tokens)".into(), "unknown".into()),
    };

    let account_id = tokens.account_id.unwrap_or_else(|| "unknown".into());

    // Try direct email first
    if let Some(ref email) = tokens.email {
        if !email.is_empty() {
            return (email.clone(), account_id);
        }
    }

    // Try JWT id_token
    if let Some(ref id_token) = tokens.id_token {
        if let Some(claims) = decode_jwt_payload(id_token) {
            if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
                return (email.to_string(), account_id);
            }
        }
    }

    ("(no email)".into(), account_id)
}

/// Decode the middle (payload) part of a JWT without verification.
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    // base64url decode with padding
    // Manually handle base64url → standard base64
    let mut standard = String::with_capacity(parts[1].len());
    for c in parts[1].chars() {
        match c {
            '-' => standard.push('+'),
            '_' => standard.push('/'),
            c => standard.push(c),
        }
    }
    while standard.len() % 4 != 0 {
        standard.push('=');
    }
    let decoded = base64_decode(&standard)?;
    serde_json::from_slice(&decoded).ok()
}

/// Minimal base64 decode (standard alphabet).
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buf = Vec::with_capacity(input.len() * 3 / 4);
    let mut accum: u32 = 0;
    let mut bits: u32 = 0;

    for ch in input.bytes() {
        if ch == b'=' {
            break;
        }
        let val = TABLE.iter().position(|&b| b == ch)? as u32;
        accum = (accum << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            buf.push((accum >> bits) as u8);
            accum &= (1 << bits) - 1;
        }
    }
    Some(buf)
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

fn codex_home() -> PathBuf {
    home_dir().join(CODEX_DIR)
}

/// Discover all accounts.
/// Scans `~/.codex-*` directories plus the default `~/.codex`.
pub fn discover() -> Vec<Account> {
    let home = home_dir();
    let codex_home = codex_home();
    let mut accounts = Vec::new();

    // Resolve what ~/.codex points to (could be symlink or real dir)
    let active_target = fs::read_link(&codex_home).ok();
    let codex_is_symlink = active_target.is_some();

    // Scan ~/.codex-<alias> dirs
    if let Ok(entries) = fs::read_dir(&home) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with(".codex-") {
                continue;
            }
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let alias = name_str.strip_prefix(".codex-").unwrap_or(&name_str);
            let path = entry.path();
            let auth_path = path.join("auth.json");
            let (email, account_id) = read_email_and_id(&auth_path);

            let active = if let Some(ref target) = active_target {
                // ~/.codex symlink → compare resolved paths
                target == &path || fs::canonicalize(&codex_home).ok() == fs::canonicalize(&path).ok()
            } else {
                false
            };

            accounts.push(Account {
                alias: alias.to_string(),
                email,
                account_id,
                path,
                active,
            });
        }
    }

    // If ~/.codex is a real directory (not symlink), include it as "default" account
    if !codex_is_symlink && codex_home.is_dir() {
        let auth_path = codex_home.join("auth.json");
        if auth_path.exists() {
            let (email, account_id) = read_email_and_id(&auth_path);
            accounts.push(Account {
                alias: "default".into(),
                email,
                account_id,
                path: codex_home.clone(),
                active: true,
            });
        }
    }

    // If ~/.codex is a symlink but its target is not in the ~/.codex-* pattern,
    // still mark the correct one as active
    if codex_is_symlink {
        if let Ok(resolved) = fs::canonicalize(&codex_home) {
            for acc in &mut accounts {
                if fs::canonicalize(&acc.path).ok() == Some(resolved.clone()) {
                    acc.active = true;
                }
            }
        }
    }

    accounts.sort_by(|a, b| a.alias.cmp(&b.alias));
    accounts
}

/// Return the currently active account, if any.
pub fn current() -> Option<Account> {
    discover().into_iter().find(|a| a.active)
}

/// Switch to the account with the given alias by repointing the ~/.codex symlink.
pub fn switch_to(alias: &str) -> Result<Account, String> {
    let accounts = discover();
    let target = accounts
        .iter()
        .find(|a| a.alias == alias)
        .ok_or_else(|| format!("no account named '{}'", alias))?
        .clone();

    let codex_home = codex_home();

    // If ~/.codex is a regular directory (first switch), rename it to ~/.codex-default
    if codex_home.exists() && codex_home.is_dir() && fs::read_link(&codex_home).is_err() {
        let default_path = home_dir().join(".codex-default");
        if !default_path.exists() {
            fs::rename(&codex_home, &default_path).map_err(|e| {
                format!("cannot rename {} to {}: {}", codex_home.display(), default_path.display(), e)
            })?;
            // Update the default account's path
            if target.alias == "default" {
                // We'll recreate the symlink below
            }
        }
    }

    // Remove existing symlink or dir
    if codex_home.is_symlink() {
        fs::remove_file(&codex_home)
            .map_err(|e| format!("cannot remove symlink: {}", e))?;
    }

    // Create symlink
    unix_fs::symlink(&target.path, &codex_home)
        .map_err(|e| format!("cannot create symlink: {}", e))?;

    Ok(target)
}

/// Import an existing CODEX_HOME directory as a named account.
pub fn import_account(name: &str, src: &Path) -> Result<Account, String> {
    if !src.is_dir() {
        return Err(format!("{} is not a directory", src.display()));
    }
    let auth_path = src.join("auth.json");
    if !auth_path.exists() {
        return Err(format!("no auth.json found in {}", src.display()));
    }

    let dest = home_dir().join(format!(".codex-{}", name));
    if dest.exists() {
        return Err(format!("{} already exists", dest.display()));
    }

    // Copy identity-defining files only (skip caches, logs, history)
    copy_dir_filtered(src, &dest)
        .map_err(|e| format!("failed to copy {} → {}: {}", src.display(), dest.display(), e))?;

    let (email, account_id) = read_email_and_id(&dest.join("auth.json"));
    Ok(Account {
        alias: name.to_string(),
        email,
        account_id,
        path: dest,
        active: false,
    })
}

/// Files/dirs to copy on import (identity & config, not heavy caches/logs).
const IMPORT_FILES: &[&str] = &[
    "auth.json",
    "config.toml",
    "version.json",
    "installation_id",
    ".personality_migration",
    "rules",
    "skills",
    "memories",
];

fn copy_dir_filtered(src: &Path, dest: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !IMPORT_FILES.contains(&name_str.as_ref()) {
            continue;
        }
        let src_path = entry.path();
        let dest_path = dest.join(&name);
        if src_path.is_dir() {
            copy_dir_filtered(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
