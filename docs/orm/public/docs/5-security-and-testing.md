# Security and Advanced Testing

Rullst ORM employs an enterprise-grade CI/CD and security pipeline. While basic checks (format, clippy, test, coverage, OSSF Scorecard) run automatically on every `push` and `pull_request`, we utilize four "Heavy" testing tools that must be triggered manually via the GitHub Actions `workflow_dispatch` button.

Because these tools are computationally expensive, it is highly recommended to execute them in a specific logical order to prevent wasting time diagnosing complex issues that a simpler tool could have caught earlier.

## Recommended Execution Order

When preparing for a major release (e.g., `v7.0`) or after rewriting a core component, trigger the workflows in this exact order:

### 1. Miri Memory Interpreter (`miri.yml`)
- **What it does:** Runs the entire test suite through the Rust Mid-level IR (MIR) to mathematically intercept Undefined Behavior (UB), Use-After-Free, and strict aliasing violations.
- **Why first?** If your code has a fundamental memory leak or unsafe pointer misuse, the other tools (especially Fuzzing and Mutation) will crash randomly or produce false positives. **Always ensure Miri passes first.**
- **Duration:** ~2 to 5 minutes.

### 2. Kani Rust Verifier (`kani.yml`)
- **What it does:** Performs Model Checking and Symbolic Execution on functions tagged with `#[kani::proof]`. It mathematically proves that specific algorithms (like our audit log masking) will *never* crash for any possible input.
- **Why second?** It validates your core logic and boundaries using mathematical proofs. If Kani fails, your logic is fundamentally flawed, and fuzzing is unnecessary.
- **Duration:** ~3 to 5 minutes (distributed via 2 shards).

### 3. Distributed Mutation Testing (`mutants.yml`)
- **What it does:** Parses the entire codebase and maliciously alters the logic (e.g., changing `==` to `!=`, deleting lines). It then runs the test suite to ensure that your tests *fail*. If the tests pass, the mutant "survived", meaning your test coverage is inadequate.
- **Why third?** Mutation testing assumes your tests are already 100% stable and memory-safe (proven by Miri). It is slightly slower, so you only run it when you know the codebase is structurally sound.
- **Duration:** ~5 to 10 minutes (distributed across 8 shards).

### 4. Distributed Fuzzing (`fuzz.yml`)
- **What it does:** Continuously generates millions of malformed, random byte combinations and blasts them against the ORM parsers (via the `fuzz/` harnesses) to find edge cases that humans couldn't think of.
- **Why last?** Fuzzing is "infinite" by nature. Even though we limited it to 300 seconds per shard, it is a brute-force approach. You should only brute-force the codebase after you have proven its logic (Kani), verified its memory (Miri), and ensured its tests are strong (Mutants).
- **Duration:** ~5 minutes (8 shards running 300 seconds each).

---

## How to Trigger
1. Go to the **Actions** tab on the GitHub repository.
2. On the left sidebar, select the desired workflow (e.g., "Mutation Testing").
3. Click the blue **Run workflow** dropdown on the right side.
4. Select the `main` branch and execute.

---

## Data Privacy & Compliance (GDPR/LGPD)

Rullst-ORM is the first Rust ORM to offer **Compile-Time Privacy & Compliance**. 
This is achieved via the `#[derive(PersonalData)]` macro and the `SecretString` type.

### How it works

When a struct derives `PersonalData` and tags fields with `#[privacy]`:
1. **At-Rest Encryption:** Using `SecretString` automatically applies AES-256-GCM encryption before saving data into the database and decrypts it when retrieving.
2. **Log Masking:** It injects a custom `std::fmt::Debug` implementation. If an object is accidentally logged (e.g. `println!("{:?}", user)`), sensitive data is masked as `[REDACTED_BY_RULLST_SHIELD]`.
3. **Automated Reports:** The model implements the `ComplianceModel` trait, allowing you to extract a `PrivacyReport` mapping which tables and fields contain encrypted personal data for GDPR/LGPD audits.

**Example usage:**
```rust
use rullst_orm::{Orm, PersonalData};
use rullst_orm::privacy::SecretString;

#[derive(Orm, PersonalData)]
#[orm(table = "users")]
pub struct User {
    pub id: i64,
    pub name: String,
    
    // Encrypted in the DB, masked in logs
    #[privacy(encrypt = "aes-256-gcm", mask = "cpf")]
    pub cpf: SecretString,
}
```

*Note: You must set the `RULLST_ENCRYPTION_KEY` (32 bytes) environment variable.*
