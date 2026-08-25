# Rullst Security 🛡️
### *"Defense-in-Depth RASP, Cryptographic Vault & Runtime Protection for Rust"*

`rullst-security` delivers military-grade runtime application self-protection, authenticated field encryption, and defensive middleware layers to protect web applications against OWASP Top 10 and API Top 10 vulnerabilities.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Rullst Vault** | 🟢 `[Production-Ready]` | Authenticated AES-256-GCM encryption with 96-bit nonces, AAD, versioned envelopes, and keyring rotation. |
| **Zero-Allocation RASP** | 🟢 `[Production-Ready]` | High-speed ASCII pattern matching for SQLi, XSS, and Path Traversal across query params and headers. |
| **Login Guard Tarpit** | 🟢 `[Production-Ready]` | Progressive async backoff delays and automatic 15-minute IP bans for repeated brute-force authentication attempts. |
| **Sliding-Window Rate Limiter** | 🟢 `[Production-Ready]` | Memory-efficient sliding-window rate limiter with automatic async janitor pruning. |
| **DLP & Secret Masking** | 🟢 `[Production-Ready]` | Automatic masking of credit cards, CPF/CNPJ documents, AWS keys, and database credentials in HTTP responses. |
| **TOTP Multi-Factor Auth** | 🟢 `[Production-Ready]` | RFC-6238 compliant 6-digit TOTP generator and validator with `otpauth` QR code builder. |
| **CSWSH Hijacking Guard** | 🟢 `[Production-Ready]` | Strict WebSocket handshake validation preventing Cross-Site WebSocket Hijacking. |
| **Distributed Rate Limiting** | 🔵 `[Roadmap]` | Cluster-wide distributed rate limiting via Redis Sentinel / Redis Cluster. |

---

## 🔐 1. Rullst Vault (Authenticated AES-256-GCM)

`FieldEncryptor` provides authenticated encryption at rest (AEAD) with built-in versioning and zero-downtime key rotation support.

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
use rullst_security::rasp::RaspInspector;

let is_sqli = RaspInspector::detect_sqli("SELECT * FROM users WHERE id = 1 OR 1=1--");
assert!(is_sqli);

let is_traversal = RaspInspector::detect_path_traversal("../../../etc/passwd");
assert!(is_traversal);
```

---

## 🚫 3. Anti-Bruteforce Login Guard & Tarpit

```rust
use rullst_security::login_guard::LoginGuard;
use std::time::Duration;

let guard = LoginGuard::new(5, Duration::from_secs(900)); // 5 max attempts, 15-min jail

// Record failed login
guard.record_failure("192.168.1.100").await;

// Check if IP is currently jailed
if guard.is_jailed("192.168.1.100").await {
    println!("IP is in Login Jail!");
}
```

---

## 📱 4. Multi-Factor Authentication (TOTP MFA)

```rust
use rullst_security::mfa::MfaEngine;

let secret = MfaEngine::generate_secret();
let otp_uri = MfaEngine::generate_otpauth_uri("alice@example.com", "Rullst SaaS", &secret);

// Validate 6-digit code submitted by user
let is_valid = MfaEngine::verify_code(&secret, "123456");
```
