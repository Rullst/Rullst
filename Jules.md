# Rullst Security & Performance Audit Report

## 1. Security Analysis

### 1.1 Findings

- **Authentication & Session Management:**
  - Uses `Argon2id` for password hashing and verification, which is the current industry standard.
  - Generates secure salts securely using `OsRng`.
  - Session cookies are securely encrypted using `AES-256-GCM` with a 12-byte random nonce, providing confidentiality and integrity.
  - Cookies are correctly flagged with `HttpOnly` and `SameSite=Lax`. In production, the `Secure` flag should also be ensured via reverse proxy or explicitly set.
  - Missing APP_KEY in production safely panics, preventing weak ephemeral keys.

- **Storage Module (Path Traversal Protection):**
  - The `LocalDriver` correctly rejects absolute paths and drive letters.
  - It explicitly blocks path traversal attempts by checking for `ParentDir` (`..`) components and normalizes paths.
  - Symlink checks are in place to ensure resolved paths remain within the `STORAGE_ROOT`.

- **Web Application Firewall (WAF) & CSRF:**
  - A custom WAF middleware (`waf_middleware`) inspects `User-Agent` strings for known bots/scrapers and query parameters for basic SQLi/XSS/Traversal patterns.
  - Security headers are properly injected (`headers_middleware`), including `X-Frame-Options`, `X-Content-Type-Options: nosniff`, and `Strict-Transport-Security`.
  - A PII masking middleware exists to redact sensitive information (Credit Cards, Emails) from response payloads.

- **HTML Templating & XSS Prevention:**
  - The `html!` macro escapes dynamic text interpolation by default via `HtmlEscape` trait (`escape_str`).
  - Standard primitives (integers, floats, booleans) are treated as safe.

- **Unsafe Code:**
  - Several `unsafe` blocks are used for FFI, specifically in `server.rs` around `libloading` for dynamic router initialization (`rullst_router_init`). These blocks are appropriately documented with their invariants.

### 1.2 Recommendations (Security)

- **Severity: Low** - The WAF middleware checks for SQLi patterns (like "select ", "union ") in query strings using a basic `.contains()` match. While helpful against automated script kiddie tools, this is rudimentary. Real protection relies on parameterized queries in the ORM. Ensure that `rullst-orm` always uses `sqlx`'s query binding mechanisms and never concatenates raw strings into SQL.
- **Severity: Low** - PII masking uses basic string heuristics. While it avoids regex overhead, it might accidentally mask valid data or miss edge-case formats. Keep this under review.

---

## 2. Performance Analysis

### 2.1 Findings

- **HTML Rendering Engine (`html!` macro):**
  - The macro attempts to pre-allocate capacity (`String::with_capacity(#capacity)`) which is an excellent optimization for reducing allocations.
  - However, for attributes, it uses `format!(" {}=\"{}\"", attr_name, val)` inside the generated code, and then appends this formatted string to the main buffer.

- **Caching:**
  - The `MemoryDriver` relies on `DashMap` for concurrent access, which is highly performant.
  - The `RedisDriver` implements `flush` using the `SCAN` cursor rather than `KEYS *`, preventing Redis event loop blocking on large datasets.
  - Methods avoid unnecessary cloning where possible, taking string slices (`&str`).

- **Clippy Analysis:**
  - Clippy reports zero errors and zero warnings across the codebase.

### 2.2 Recommendations (Performance)

- **Severity: Medium - HTML Macro Attribute Allocations:**
  - In `rullst-macros/src/html_parser.rs`, the code generation for attributes uses `format!` which creates a new intermediate `String` allocation for every single attribute of every element.
  - *Fix:* Instead of `format!`, the macro should generate calls to `s.push_str(" attr=\"");`, `s.push_str(val);`, and `s.push_str("\"");` directly onto the pre-allocated string `s`. This aligns with the "Performant string building in loops" guideline and eliminates hundreds of tiny allocations during page rendering.

- **Severity: Low - Unnecessary Clones:**
  - There are instances of `.clone()` on strings and Arcs (e.g., in `resilience.rs`, `nexus.rs`, `testing.rs`). While most are on small strings or `Arc`s (which is cheap), evaluating hot paths (like middleware `clone()` on every request) could yield minor throughput improvements.
