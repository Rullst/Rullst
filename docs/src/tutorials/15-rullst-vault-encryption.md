# Tutorial 15: Rullst Vault & Field Encryption 🔐

Learn how to encrypt database fields using transparent AES-256-GCM / ChaCha20-Poly1305 and protect secrets in RAM using `Zeroize`.

---

## 🛠️ Step 1: Encrypt Database Model Fields

In `src/models/user.rs`:

```rust
use rullst_orm::prelude::*;

#[derive(Debug, Serialize, Deserialize, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: PrimaryKey<i64>,
    pub email: String,
    
    // Transparent field-level database encryption
    #[orm(encrypted)]
    pub ssn_tax_id: String,
}
```

---

## 🔒 Step 2: Protect Secrets in Memory (`VaultSecret`)

```rust
use rullst_security::vault::VaultSecret;

fn process_payment_api_key() {
    // Secret is zeroed out in RAM immediately when dropped
    let api_key = VaultSecret::new("sk_live_998877665544332211".to_string());
    
    println!("Key loaded into secure memory scope.");
    // Automatic zeroize execution on drop
}
```

---

## 💡 Key Takeaways
- `#[orm(encrypted)]` transparently encrypts strings before SQL `INSERT`/`UPDATE` and decrypts on `SELECT`.
- `VaultSecret<T>` prevents secret leakages in heap dumps during crash memory dumps.
