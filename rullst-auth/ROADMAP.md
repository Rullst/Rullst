# Rullst Auth - Roadmap

Rullst Auth is the core security and authentication module for the framework. While we already support Social Auth (`rullst-connect`), Passkeys (WebAuthn), and Local Auth (Sessions/JWT), the journey to enterprise-grade security continues.

## Phase 1: Access Control & Permissions
- [ ] **Role-Based Access Control (RBAC)**: A native system to assign roles (e.g., `Admin`, `Editor`, `User`) and attach them seamlessly to routes via middleware (`#[require_role("Admin")]`).
- [ ] **Declarative Policies (Gates)**: Define granular authorization logic in Rust structs (e.g., `PostPolicy::can_edit(&user, &post)`) that can be invoked across controllers and templates.

## Phase 2: Advanced Verification
- [ ] **Two-Factor Authentication (2FA)**: Built-in TOTP generation and validation for Google Authenticator/Authy integration, including recovery codes.
- [ ] **Magic Links**: Passwordless authentication via single-use, time-sensitive signed URLs delivered via email (`rullst-mail` integration).
- [ ] **Device Management**: Track active sessions across devices and provide an API for users to "Sign out of all other sessions".
