# Rullst Auth - Roadmap

> **Status policy (2026-08-26):** the ideas below are preserved, but legacy
> checkboxes are not broader production guarantees. See the audited
> [`rullst-auth` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps) and the
> [capability ledger](../docs/src/capability-ledger.md) for current boundaries.

Rullst Auth is the core security and authentication module for the framework. While we already support Social Auth (`rullst-connect`), Passkeys (WebAuthn), and Local Auth (Sessions/JWT), the journey to enterprise-grade security continues.

## Phase 1: Access Control & Permissions
- [x] **Role-Based Access Control (RBAC)**: A native system to assign roles (e.g., `Admin`, `Editor`, `User`) and attach them seamlessly to routes via middleware (`#[require_role("Admin")]`).
- [x] **Declarative Policies (Gates)**: Define granular authorization logic in Rust structs (e.g., `PostPolicy::can_edit(&user, &post)`) that can be invoked across controllers and templates.

The verified v12 boundary includes fail-closed `RequireRoleLayer`, the facade
attribute for async handlers with an explicit authenticated `user` binding, and
named `Policy<User, Resource>` structs. Applications still own role persistence,
authentication and tenant/resource lookup.

## Phase 2: Advanced Verification
- [ ] **Two-Factor Authentication (2FA)**: Built-in TOTP generation and validation for Google Authenticator/Authy integration, including recovery codes.
- [ ] **Magic Links**: Passwordless authentication via single-use, time-sensitive signed URLs delivered via email (`rullst-mail` integration).
- [ ] **Device Management**: Track active sessions across devices and provide an API for users to "Sign out of all other sessions".

The v12 `sqlite` profile now covers a bounded subset of device management:
passkey registration, inventory, rename, revocation, restart persistence and
signature-counter CAS, plus shared-local JWT revocation by token or subject
session version. It does not inventory cookie/refresh sessions, authenticate
device ownership, share WebAuthn challenges across instances, replicate across
hosts or implement the complete "sign out all other sessions" product flow.
