# Security Policy

## Supported Versions

Rullst is currently in active development. Only the latest major release (v4.x) receives security updates.

| Version | Supported          |
| ------- | ------------------ |
| 4.x.x   | :white_check_mark: |
| < 4.0.0 | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability within Rullst, please DO NOT open a public issue. Instead, send an email to the core maintainers team at rullst.rullst@creio.eu.

Please include the following information in your report:
- The type of vulnerability (e.g., XSS, SQLi, CSRF, Path Traversal).
- The steps to reproduce the vulnerability.
- Any potential impact on user data or server integrity.

All security vulnerabilities will be promptly addressed. We practice coordinated disclosure and will credit you in the release notes if your report leads to a patch.

## Security Posture

Rullst adopts a "Secure by Design" philosophy and runs continuous automated security pipelines (DAST via OWASP ZAP, SAST via Cargo Deny, and Fuzzing) to prevent regressions. Please refer to our `audit.md` for our latest comprehensive Code Audit report.
