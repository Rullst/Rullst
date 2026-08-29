# Rullst Security 🛡️
### *"Defense-in-Depth RASP, Cryptographic Vault & Runtime Protection for Rust"*

`rullst-security` provides authenticated field encryption, bounded defensive
middleware, local abuse controls, and security telemetry. Its RASP/DLP rules are
defense-in-depth heuristics: they reduce specific risks but do not establish
complete OWASP coverage, replace parameterized SQL/authorization, or certify the
application that mounts them.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Rullst Vault** | 🟢 `[Implemented]` | Authenticated AES-256-GCM encryption with 96-bit nonces, AAD, versioned envelopes, and keyring rotation. Key custody remains external. |
| **Bounded RASP** | 🟢 `[Implemented: defense in depth]` | ASCII signature matching plus one decoding pass for URI, headers, and bounded textual/JSON bodies. Decoding and body inspection may allocate. |
| **Login Guard Tarpit** | 🟢 `[Implemented: local]` | Progressive delay decisions and bounded, expiring in-memory jails keyed by a hashed identity. The caller performs the returned delay. |
| **Sliding-Window Rate Limiter** | 🟢 `[Implemented: local]` | In-memory limiter keyed from the verified socket peer. It does not coordinate multiple processes. |
| **Redis Rate Limiter** | 🟢 `[Implemented: feature-gated foundation]` | `redis-rate-limit` uses an atomic fixed-window Lua script, namespace validation, hashed client keys and TTL-derived retry metadata. Empty/`mock_*` URLs select an explicit process-local test mode; call `require_distributed()` at production startup. A live contract proves independent clients share one Redis budget; cluster/failover remains application evidence. |
| **DLP & Secret Masking** | 🟢 `[Implemented: bounded]` | Masks complete private-key envelopes, AWS access-key patterns, and credentials in supported textual database URLs. Binary, compressed, streaming, unknown-size, and oversized bodies are not rewritten. |
| **TOTP Multi-Factor Auth** | 🟢 `[Implemented: foundation]` | Six-digit SHA-1 TOTP generation/verification with a ±1 time-step window, percent-encoded `otpauth` URI builder, and subject-bound single-use recovery-code verifiers. Enrollment, transactional persistence, rate limits, and account policy remain application concerns. |
| **CSWSH Guard** | 🟢 `[Implemented]` | Exact normalized scheme/host/port validation for WebSocket origins, with a fail-closed default for missing origins. |
| **Canonical Server security stack** | 🟡 `[Partial]` | CSP nonce identity is shared across Core and extended layers, but Core still owns the default Server CSRF/WAF/header/PII stack. Explicit composition is required. |
| **Distributed Rate Limiting Evidence** | 🟡 `[Partial]` | The Redis adapter is implemented, but real cross-instance, eviction/failover and trusted-proxy deployment tests remain required. The legacy no-argument distributed selector still returns `Unsupported` rather than guessing configuration. |

---

## 🔐 1. Rullst Vault (Authenticated AES-256-GCM)

`FieldEncryptor` provides authenticated encryption at rest (AEAD) with a
versioned envelope and keyring-assisted rotation. Applications can keep prior
keys readable while writing with a new key; deployment coordination, key
custody, data re-encryption, and retirement remain operator responsibilities.

### Usage Example

```rust
use rullst_security::vault::FieldEncryptor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let master_key = [0x42u8; 32]; // 256-bit cryptographic key
    let sensitive_data = "user_ssn_123-45-6789";

    // Encrypt with key ID and Additional Authenticated Data (AAD)
    let encrypted = FieldEncryptor::encrypt_with_key_id(
        sensitive_data,
        master_key,
        "key-2026-v1",
        b"tenant-organization-id-42",
    )?;
    println!("Ciphertext Envelope: {}", encrypted);

    // Decrypt and verify authentication tag and AAD
    let decrypted = FieldEncryptor::decrypt_with_aad(
        &encrypted,
        master_key,
        b"tenant-organization-id-42",
    )?;
    assert_eq!(decrypted, sensitive_data);

    Ok(())
}
```

---

## 🛡️ 2. Runtime Application Self-Protection (RASP)

The RASP request inspector scrutinizes incoming requests before they reach your controllers:

```rust
use rullst_security::RaspInspector;

let is_sqli = RaspInspector::inspect_text("admin' OR '1'='1");
assert!(is_sqli);

let is_traversal = RaspInspector::inspect_uri("/files/../../../etc/passwd");
assert!(is_traversal);
```

Mount `RaspSecurityLayer` explicitly when the extended inspector is desired.
The default `rullst-core::Server` currently mounts the smaller Core WAF rather
than this layer; consolidation remains roadmap work.

---

## 🚫 3. Anti-Bruteforce Login Guard & Tarpit

```rust
use rullst_security::LoginGuard;

let guard = LoginGuard::new(); // defaults: 5 failures, 15-minute local jail

// Record failure and apply the returned progressive delay in the async handler.
let delay = guard.record_login_failure("account:alice");
tokio::time::sleep(delay).await;

if guard.is_jailed("account:alice") {
    println!("Identity is in the local Login Jail");
}
```

---

## 📱 4. Multi-Factor Authentication (TOTP MFA)

```rust
use rullst_security::{
    build_otpauth_uri, generate_mfa_secret, generate_totp_code, verify_totp_code,
};

let secret = generate_mfa_secret();
let otp_uri = build_otpauth_uri("Rullst SaaS", "alice@example.com", &secret);
let current_code = generate_totp_code(&secret);

// Validate 6-digit code submitted by user
let is_valid = verify_totp_code(&secret, "123456");
```

The secret must be encrypted at rest. Recovery helpers return plaintext codes
only at enrollment and salted HMAC verifiers for storage; consume/delete must be
one durable transaction. The application still owns replay/attempt limiting,
enrollment confirmation, recovery UX, audit and clock-monitoring policy.
