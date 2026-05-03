# Auth extraction

`read_email_and_id()` in `src/account.rs:35-70` extracts the email address and account ID from a Codex `auth.json` file. This is what identifies each account in `list` and `current` output.

## auth.json structure

```json
{
  "auth_mode": "chatgpt",
  "OPENAI_API_KEY": null,
  "tokens": {
    "id_token": "eyJhbGci...<JWT>...",
    "access_token": "eyJhbGci...<JWT>...",
    "refresh_token": "rt_...",
    "account_id": "9cf65c60-48b1-4b90-9b65-2adbf22a53d0",
    "email": "user@example.com"
  },
  "last_refresh": "2026-05-01T09:43:01Z"
}
```

Only two fields matter: `tokens.email` and `tokens.id_token`. Everything else is ignored.

## Extraction strategy

The function tries two sources in order:

### 1. Direct email field

```rust
if let Some(ref email) = tokens.email {
    if !email.is_empty() {
        return (email.clone(), account_id);
    }
}
```

Some Codex versions store email directly in `tokens.email`. If present and non-empty, this is used immediately.

### 2. JWT id_token payload

If the direct email field is absent or empty, the function falls back to decoding the `id_token` JWT:

```rust
if let Some(ref id_token) = tokens.id_token {
    if let Some(claims) = decode_jwt_payload(id_token) {
        if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
            return (email.to_string(), account_id);
        }
    }
}
```

The JWT `id_token` is an OpenID Connect token whose payload (the middle base64url-encoded segment) contains identity claims including `email`.

## JWT payload decoding

`decode_jwt_payload()` at `src/account.rs:73-93` decodes the JWT payload **without verifying the signature**. This is intentional — we only need identity metadata, not proof of authentication. Verification would require knowing Codex's signing public key, which we don't have and don't need.

### Step 1: Split the JWT

```
eyJhbGciOiJSUzI1NiIs... . eyJhdWQiOlsiaHR0cHM6L... . signature
       HEADER                  PAYLOAD (base64url)
```

```rust
let parts: Vec<&str> = token.split('.').collect();
// parts[0] = header, parts[1] = payload, parts[2] = signature
```

### Step 2: Convert base64url → standard base64

JWTs use base64url encoding: `-` instead of `+`, `_` instead of `/`, and no `=` padding.

```rust
let mut standard = String::with_capacity(parts[1].len());
for c in parts[1].chars() {
    match c {
        '-' => standard.push('+'),
        '_' => standard.push('/'),
        c => standard.push(c),
    }
}
while standard.len() % 4 != 0 {
    standard.push('=');  // restore padding
}
```

### Step 3: Base64 decode

`base64_decode()` at `src/account.rs:96-116` is a hand-rolled standard base64 decoder using a 6-bit accumulator. It processes input bytes in groups of 4, producing 3 output bytes per group. Padding `=` terminates decoding early.

```
Base64 alphabet: A-Z (0-25), a-z (26-51), 0-9 (52-61), + (62), / (63)
```

### Step 4: Parse as JSON

The decoded payload bytes are parsed with `serde_json::from_slice`. The resulting JSON object contains standard OpenID Connect claims:

```json
{
  "aud": ["app_EMoamEEZ73f0CkXaXp7hrann"],
  "email": "xuranus@protonmail.com",
  "email_verified": true,
  "iat": 1777628580,
  "iss": "https://auth.openai.com",
  "sub": "auth0|dHtq..."
}
```

We only extract `email`. The `iat`, `iss`, `sub`, and other claims are available but unused.

## account_id

The `account_id` is a UUID (`tokens.account_id`) that identifies the Codex/OpenAI account. It's read directly from the JSON — no decoding needed:

```rust
let account_id = tokens.account_id.unwrap_or_else(|| "unknown".into());
```

It's displayed in truncated form (first 8 chars) by `list`:

```
→ personal         xuranus42@qq.com               b061a30a
```

## Failure modes

| Condition | Result |
|-----------|--------|
| No `auth.json` file | email = `"(no auth.json)"` |
| Invalid JSON | email = `"(invalid json)"` |
| No `tokens` key | email = `"(no tokens)"` |
| No `email` field and no `id_token` | email = `"(no email)"` |
| JWT payload not valid JSON | Falls through, email = `"(no email)"` |

In all failure cases, the account is still listed — it just won't have a human-readable email. Switching still works, since switching only cares about the alias (directory name), not the email.
