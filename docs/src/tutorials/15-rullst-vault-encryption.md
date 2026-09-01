# Tutorial 15: Vault and ORM field encryption

Rullst v12 can encrypt `String` and `Option<String>` model fields before they
reach the database and decrypt them when a generated ORM query loads the model.
The implementation uses AES-256-GCM with a fresh random nonce and an
authenticated, versioned envelope.

## 1. Configure a current key

Generate 32 random bytes and store them as a prefixed base64 value. Do not
commit this value to source control:

```bash
export RULLST_ENCRYPTION_KEY="base64:$(openssl rand -base64 32)"
export RULLST_ENCRYPTION_KEY_ID="production-2026-01"
```

`RULLST_ENCRYPTION_KEY` accepts `base64:<value>`, `hex:<value>`, or a legacy
raw value containing exactly 32 UTF-8 bytes. A secret manager or KMS-backed
deployment adapter should inject it at runtime. Rullst does not provide key
custody.

## 2. Mark model fields

```rust
use rullst_orm::FromRow;

#[derive(Clone, Debug, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub email: String,
    #[orm(encrypted)]
    pub tax_id: String,
    #[orm(encrypted)]
    pub recovery_note: Option<String>,
}
```

Generated `save`, `update_partial`, `find`, `all`, query-builder loads and
streams preserve plaintext in the Rust model while storing an envelope shaped
like this in the SQL column:

```text
RULLST:v2:<key_id>:<base64url_nonce>:<base64url_ciphertext_and_tag>
```

The table and column names are authenticated as additional data. Copying a
ciphertext into a different annotated column therefore fails decryption.

## 3. Rotate a key without downtime

Set the new current key and keep old readable keys in a JSON keyring:

```bash
export RULLST_ENCRYPTION_KEY="base64:<new-32-byte-key>"
export RULLST_ENCRYPTION_KEY_ID="production-2027-01"
export RULLST_ENCRYPTION_KEYRING='{
  "production-2026-01": "base64:<old-32-byte-key>"
}'
```

Reads select the key named by the envelope. A normal full `save()` rewrites all
annotated values with the current key. Keep every key needed by existing rows
until a separately monitored migration has rewritten and verified them; then
remove the retired key.

## 4. Understand query limits

AES-GCM encryption is randomized, so the same plaintext produces different
ciphertexts. Generated queries reject encrypted fields in `WHERE`, `ORDER BY`,
`GROUP BY`, and explicit `SELECT` clauses rather than silently returning wrong
results. `pluck_string` supports non-null encrypted strings; load the model for
nullable encrypted strings.

For lookup, add a separate application-designed blind-index column and assess
its equality-leakage and key-rotation trade-offs. Rullst v12 does not generate a
blind index automatically. Raw SQL is an explicit escape hatch and does not
automatically encrypt bindings or decrypt arbitrary projections.

## 5. Reduce secret lifetime in memory

`VaultSecret<T>` redacts `Debug`/`Display` and calls `Zeroize` on the wrapped
value when dropped:

```rust
use rullst_security::VaultSecret;

fn use_api_key() {
    let Ok(value) = std::env::var("EXAMPLE_PROVIDER_KEY") else {
        return;
    };
    let api_key = VaultSecret::new(value);
    send_request(api_key.expose_secret());
}

# fn send_request(_: &str) {}
```

Zeroization is defense in depth, not secure memory. It cannot erase prior
copies or prevent a debugger, core dump, swap, allocator behavior, or another
process-memory capture while the secret is live.

## Operational checklist

- Back up the database and keyring together, with separate access controls.
- Test restore and rotation before retiring any key.
- Restrict environment and crash-dump access.
- Never log plaintext model fields or expose them through serialization by
  accident; add `#[orm(hidden)]` when a field must be omitted from generated
  `to_json()` output.
- Treat authentication failure as possible corruption, wrong context, wrong
  key, or tampering; do not replace it with an empty value.
