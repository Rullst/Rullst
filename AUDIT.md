# Rullst Framework - Security & Performance Audit Report

## 1. Executive Summary
This document provides a comprehensive security and performance audit of the `rullst` framework workspace. The audit covered major components including routing, the active record ORM (`rullst-orm`), CLI generators (`cargo-rullst`), and core security middleware (`rullst::security`).

Overall, the framework makes strong use of Rust's type safety and memory-safety guarantees. However, a few critical security design flaws, memory-safety risks in FFI boundaries, performance bottlenecks in database querying, and dependency lifecycle issues were identified.

---

## 2. Security Vulnerabilities

### 2.1 Critical: Missing WAF and CSRF Middleware in Server Router
- **Location:** `rullst/src/server.rs`
- **Issue:** The framework defines robust `waf_middleware` and `csrf_middleware` in `rullst/src/security.rs`. However, these are **never automatically attached** to the application router inside the `Server::run` execution path, even though the `.ai-rules` strictly dictates: *"WAF and CSRF middleware are mandatory for all production endpoints."*
- **Impact:** Production endpoints are exposed to bot scraping, path traversal, and Cross-Site Request Forgery (CSRF) by default.
- **Recommendation:** Integrate `.layer(axum::middleware::from_fn(crate::security::waf_middleware))` and the CSRF equivalent into `Server::run` inside `rullst/src/server.rs` when `APP_ENV` is set to `production`.

### 2.2 High: SQL Injection Vulnerabilities in `nexus.rs`
- **Location:** `rullst/src/nexus.rs` (Lines 790, 801, 803, 388, 480)
- **Issue:** The AI-native admin panel dynamically constructs SQL query strings. While it binds values via `query.bind()`, the structural components of the query (such as table names, field names in `ILIKE`/`LIKE` filters, and `COUNT` wrappers) are directly interpolated using `format!()` without strict alphanumeric sanitization.
- **Impact:** If an attacker can manipulate table or field names (e.g., via Nexus schema models), they could inject arbitrary SQL into the query structure.
- **Recommendation:** Implement strict alphanumeric and underscore validation for all table names and field names before interpolating them into `format!()` macros within the `nexus.rs` and `studio.rs` modules.

### 2.3 High: Unmaintained Vulnerable Dependency
- **Location:** `Cargo.lock` / `rullst-macros/Cargo.toml`
- **Issue:** The `cargo audit` command detected an unmaintained crate: `proc-macro-error2 v2.0.1` (RUSTSEC-2026-0173).
- **Impact:** Unmaintained macro dependencies could introduce compiler exploits or lack support for future Rust versions, breaking the compilation of `rullst-macros`.
- **Recommendation:** Refactor the `rullst-macros` crate to use the standard `syn::Error::into_compile_error()` pattern instead of relying on `proc-macro-error2`.

### 2.4 Medium: Hardcoded Environment Variable in Production Safe-Path
- **Location:** `rullst/src/storage.rs`
- **Issue:** In the `LocalDriver::resolve_path` method, directory traversal is prevented, but the root path relies on `std::env::var("STORAGE_ROOT").unwrap_or_else(|_| "storage/app".to_string())`. If an attacker can influence the environment variables before the server starts, they could redirect storage to `/etc` or other sensitive locations.
- **Recommendation:** Ensure `STORAGE_ROOT` is strictly loaded from the validated `Rullst.toml` configuration rather than direct environment variable lookups.

---

## 3. Performance Bottlenecks & Code Quality

### 3.1 High: Widespread Panic and `unwrap()` usage
- **Location:** Across the codebase (`cargo-rullst/src/generators/desktop.rs`, `rullst/src/auth.rs`, `examples/blog/src-tauri/src/main.rs`).
- **Issue:** The codebase contains numerous `.unwrap()` and `.expect()` calls in non-test paths. For instance, in `cargo-rullst`, spawned processes and thread mutexes frequently use `.expect()`.
- **Impact:** A simple failure in an OS process or a poisoned mutex will panic the entire application thread, causing unexpected crashes.
- **Recommendation:** Refactor to return `Result<T, AppError>` and bubble up errors gracefully using the `?` operator.

### 3.2 Medium: Zombie Processes in CLI Generators
- **Location:** `cargo-rullst/src/generators/desktop.rs:504`
- **Issue:** A spawned `cargo run` backend process is never `wait()`ed on or explicitly killed when the Tauri frontend exits.
- **Impact:** Repeatedly launching the desktop generator will leave orphaned zombie processes consuming RAM and occupying HTTP ports (e.g., port 3000), leading to "Address already in use" errors on subsequent runs.
- **Recommendation:** Capture the `Child` process object and call `child.kill()` or `child.wait()` in a cleanup block or `Drop` implementation.

### 3.3 Medium: Inefficient Memory Allocations in `format!`
- **Location:** `cargo-rullst/src/ui/components.rs`, `rullst/src/html.rs`
- **Issue:** Several places use `format!()` without any format arguments (e.g., `format!("🔙  Back to Main Menu        ")`) or utilize `vec![]` where a static array slice `[]` would suffice.
- **Impact:** Unnecessary heap allocations decrease throughput and increase GC pressure (in context of OS memory manager).
- **Recommendation:** Use `.to_string()` for static strings and static arrays `[...]` instead of `vec![...]`.

### 3.4 Low: Hot Reloading Dynamic Library Leaks
- **Location:** `rullst/src/server.rs:536`
- **Issue:** The `load_dylib_router` function uses `uuid::Uuid::new_v4()` to generate unique shared objects (`.so`/`.dylib`) to avoid file locks, but the garbage collection of old libraries only happens on startup.
- **Impact:** During long development sessions with Hot Reloading, the `target/debug` directory will fill up with thousands of orphaned `_active_uuid.so` files, consuming disk space.
- **Recommendation:** Keep a reference to the active temp file path and delete it in a `Drop` implementation or during the shutdown signal handler.

---

## 4. Action Plan for Remediation

1. **Update `server.rs`:** Inject `waf_middleware` and `csrf_middleware` into the Axum router builder sequence.
2. **Patch SQL injection vectors:** Introduce a `validate_identifier(name: &str)` function in `rullst/src/nexus.rs` that ensures table and field names contain only `[a-zA-Z0-9_]`.
3. **Remove `proc-macro-error2`:** Update `rullst-macros/Cargo.toml` and refactor error handling to use native `syn::Error`.
4. **Fix Clippy Warnings:** Address manual `flatten()` loops, wait on spawned processes, and replace empty `format!()` calls with `.to_string()`.
5. **Eliminate Panics:** Audit and replace `unwrap()`/`expect()` in application routes with `Result` based error handling.

*Audit performed by Jules.*
