# Security Policy

## Supported Versions

Only the latest version of `rullst-orm` is currently supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 6.x.x   | :white_check_mark: |
| < 6.0   | :x:                |

## What Constitutes a Vulnerability

We consider a security vulnerability to be any flaw or weakness in `rullst-orm` that could allow an attacker to compromise the integrity, confidentiality, or availability of an application using the ORM. This includes, but is not limited to:
- SQL Injection vectors or bypasses.
- Denial of Service (DoS) attacks (e.g., memory exhaustion or connection pool hanging).
- Unauthorized disclosure of sensitive data (e.g., flawed audit logs or model serialization).
- Cross-Site Scripting (XSS) or other web vulnerabilities within the Admin Dashboard.

## Reporting a Vulnerability

Please do NOT report security vulnerabilities via public GitHub issues.

If you believe you have found a security vulnerability in `rullst-orm`, please report it by emailing the maintainer directly at:
[rullst@creio.eu](mailto:rullst@creio.eu)

Alternatively, you can report it via a Private GitHub Security Advisory at the following URL:
[GitHub Security Advisories](https://github.com/Rullst/rullst-orm/security/advisories/new)

We take all vulnerability reports seriously and follow a coordinated disclosure process. We will try to respond to your report within 48 hours, and we aim to resolve all critical vulnerabilities within 90 days.
