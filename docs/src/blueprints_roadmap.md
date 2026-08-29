# Rullst Blueprints Roadmap 🗺️
### *"A practical collection of reviewed Rullst application starters"*

This document maps the expansion plan for the Rullst **Starter Blueprints** ecosystem. The goal is to provide reviewable application starting points with explicit production checklists and capability boundaries.

---

## 🚀 Blueprints Design Philosophy
Every blueprint added to the CLI must meet three fundamental principles:
1. **Clear first experience:** Responsive, accessible interfaces whose browser assets and CSP are explicit.
2. **Native Rust/Rullst Features:** Demonstrate measured resource use, typed concurrency and explicit server/realtime boundaries.
3. **Production-minded defaults:** Generate `.env.example`, database configuration, and a conservative `.gitignore`; deployment readiness remains an application-level review.

---

## 🗺️ Proposed blueprints (ordered from easiest to hardest)

Except for ERP, these rows describe design targets and are not selectable CLI
blueprints. A proposal becomes implemented only when its generated project and
negative boundaries pass the release gates.

| ID | Blueprint Name | Technical Focus in Rullst | Commercial Differentiator |
|:---|:---|:---|:---|
| **4** | 💼 ERP Pocket (Inventory) | Embedded SQLite + `rullst::nexus` (Auto-CMS) + Single Binary | Small/medium-business inventory starter; crash recovery and backup remain application work. |
| **5** | 📋 Member/Club Management | Validation + Nexus + reviewed receipt adapter | Proposed member and billing domain starter. |
| **7** | 🤖 AI Agent & RAG Boilerplate | `rullst-ai` + opt-in document parsing/embedding adapters | Proposed RAG starter; uploaded content remains untrusted. |
| **8** | 🪙 AI Credit-Based SaaS | SSE + transactional usage ledger + payment adapter | Proposed AI SaaS starter with server-owned credit reservation. |
| **9** | 🏥 Scheduling & Clinics | HTMX calendar + scheduler + database conflict policy | Proposed scheduling starter with database-specific contention tests. |
| **10**| 🚪 Biometric Access Control | WebSocket foundation + real-time concierge panel | Planned access-control starter; device trust and latency need deployment-specific validation. |
| **11**| 📈 Affiliate Checkout | SSR + commission-split domain model + landing page | Planned sales starter; performance and Lighthouse scores must be measured per application. |
| **12**| 🏢 B2B Multi-Tenant Platform | Tenant context + RBAC + `rullst-mail` | Planned B2B starter; isolation must be proven across every storage and messaging boundary. |
| **13**| 💬 Discord-Like Realtime Chat | Server-driven UI + authenticated WebSockets | Proposed chat starter; distributed presence and load evidence are required. |
| **14**| 🛵 Delivery / Food App | Background queue + explicit order state machine | Proposed delivery starter with idempotent jobs and notification adapters. |

---

## 🔍 Highlighted Architectural Details

### 🪙 8. AI Credit-Based SaaS (The Token-Burner)
* **Architecture goal:** a server-owned chat flow with bounded SSE streaming,
  cancellation and provider error handling.
* **Data Security goal:** use a database transaction and provider-specific lock
  semantics to reserve credit before an LLM request. This workflow is not yet a
  generated, cross-database guarantee.
* **Monetization goal:** integrate a reviewed usage ledger with a supported
  payment adapter; billing portals remain provider/application work.

### 🏢 12. B2B Multi-Tenant Platform (The Corporate Boilerplate)
* **Isolation goal:** derive a validated `TenantContext` at the HTTP boundary and
  carry it explicitly through database, cache, queue and realtime operations.
  Rullst does not inject a tenant predicate into every arbitrary SQL statement.
* **Permissions goal:** typed roles (`Admin`, `Member`, `Billing`) enforced in
  middleware and again at sensitive service boundaries.
* **Invitations goal:** hashed, single-use, expiring invitation tokens delivered
  through a configured mail adapter.

### 💬 13. Discord-Like Realtime Chat
* **Client goal:** use server-rendered messages and an explicit WebSocket client
  without making bundle size a proxy for correctness.
* **Scale gate:** publish connection count, message mix, backpressure behavior,
  CPU/RSS, hardware and distributed topology before attaching capacity numbers.

### 🏥 9. Scheduling & Clinics (The Scheduler)
* **Conflict-prevention goal:** enforce a database constraint plus a transaction
  strategy tested on every declared backend; isolation level alone is not a
  universal double-booking proof.
* **Reminder goal:** use bounded, idempotent scheduler jobs and a configured mail
  adapter. Multi-instance leadership and durable retry must be explicit.

### 🤖 7. AI Agent & RAG Boilerplate (AI-Native)
* **Structure goal:** quarantine and scan bounded uploads, parse supported formats
  in an isolated adapter, generate embeddings and store an authorized index.
* **Provider goal:** expose an explicit provider selection and capability check.
  Changing an environment variable does not make data policy, schema or model
  behavior interchangeable.
