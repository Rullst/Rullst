# Legal & Regulatory Compliance Roadmap

> **Not legal advice.** This page is an engineering roadmap written by the
> framework maintainers. It does not create a lawyer–client relationship and is
> not a statement that any application built with Rullst complies with any law.
> Every item below must be validated by qualified counsel in each jurisdiction
> before it is described as satisfying a legal requirement.

Status: **planned (recorded 2026-10-01)** for the v13 feature line. Nothing on
this page is a shipped capability unless it links to existing documentation.

## Goal and honest boundary

A framework cannot make an application lawful. Compliance depends on what the
operator collects, why, where it is stored, which contracts and notices exist,
who the users are and which country's law applies. What Rullst *can* do is make
the default path **compliance-ready**:

1. ship building blocks that implement common obligations correctly
   (consent, data-subject rights, retention, minimization, security, audit);
2. generate starters whose defaults do not create avoidable legal risk
   (no unnecessary personal data, no non-essential cookies, no PII in logs);
3. document, per regulation, which obligations a Rullst feature helps with and
   which ones remain the operator's responsibility; and
4. keep the framework's own posture clear (MIT license and warranty
   disclaimer, SBOM, coordinated vulnerability disclosure).

## Priority jurisdictions

The first guide covers the regulations most likely to affect Rullst users.
Each row must later link to the official text and be reviewed by counsel.

| Region | Main instruments | Areas where Rullst can help |
| :--- | :--- | :--- |
| European Union | GDPR; ePrivacy Directive (cookies); AI Act (transparency obligations); Cyber Resilience Act; European Accessibility Act; Digital Services Act (platforms); NIS2 | Lawful-basis and consent records, data-subject rights, records of processing, breach-response evidence, cookie defaults, AI content disclosure, SBOM and vulnerability handling, accessible starters |
| United States | State privacy laws (e.g. CCPA/CPRA); COPPA; FERPA (education records); HIPAA and GLBA (sector-specific); ADA accessibility; CAN-SPAM; FTC Act; state breach-notification and biometric laws (e.g. Illinois BIPA) | Opt-out and deletion flows, children's data gates, education-record handling in the LMS starter, unsubscribe headers, accessibility, biometric-free supervision defaults |
| Brazil | LGPD and ANPD regulations (incl. incident reporting); Marco Civil da Internet (application access-log retention); Código de Defesa do Consumidor and e-commerce rules; ECA Digital (children and adolescents online); Lei Brasileira de Inclusão (accessibility) | Consent and rights, incident timelines, configurable access-log retention, consumer information pages, age assurance and parental supervision, accessible starters, NFS-e fiscal preparation |
| Asia | China PIPL, Data Security Law and Cybersecurity Law (localization and cross-border transfer); Japan APPI; South Korea PIPA; India DPDP Act and Rules; Singapore PDPA; others (Indonesia, Vietnam, Thailand) | Data-residency configuration, cross-border transfer inventory, consent and rights flows, retention, breach-response evidence |
| Global / industry | PCI DSS (payment cards); OWASP ASVS; open-source license obligations; export controls on cryptography | Hosted-checkout-only payment flows (no card data stored), security baseline mapping, SBOM and license audit |

## What already exists (v13 working source)

These are building blocks, not compliance claims; see each page for limits:

- `rullst-privacy`: proportional age assurance, consent/withdrawal state and a
  bounded own-account profile export
  ([roadmap and scope](privacy-age-assurance-roadmap.md)).
- ORM field encryption and masking, `SecretString`, `#[privacy]` personal-data
  tagging and `ComplianceModel::compliance_schema()` / `personal_fields()`.
- Security baseline: CSRF, secure headers and CSP, WAF/RASP, DLP and PII
  masking on responses, audit logging, rate limiting
  ([security architecture](security-architecture.md)).
- Mail: List-Unsubscribe (RFC 8058), recipient suppression, minimized delivery
  observations ([mail crate](crates/mail.md)).
- Capital: provider-hosted checkout and webhook verification. Fiscal documents
  such as NFS-e are outside the framework since v13.
- Supervision: no media models or biometric identification are included; the
  host owns capture permission and reviewer workflow.
- Supply chain: SBOM generation, license policy (`deny.toml`) and coordinated
  vulnerability disclosure.

## Planned work

| # | Item | Notes |
| :--- | :--- | :--- |
| 1 | **Compliance guide** in the book, one section per regulation above | Obligation → Rullst feature → operator responsibility → official source; reviewed by counsel before publication as guidance |
| 2 | **Records of processing / data inventory** generated from `#[privacy]` tags and `compliance_schema()` | e.g. a `cargo rullst privacy:inventory` report (fields, purposes, retention, processors, transfer regions) |
| 3 | **Data-subject rights toolkit** | Access/portability export, rectification, deletion and anonymization across models, with audit trail and deadlines |
| 4 | **Retention policies** | Declarative per-model retention, scheduled purge/anonymization, configurable access-log retention (e.g. Marco Civil) |
| 5 | **Cookie and tracking defaults** | Starters keep strictly necessary cookies only; an optional consent component gates analytics and marketing |
| 6 | **Incident-response support** | Security-event export and a breach timeline template (e.g. 72 h under GDPR, ANPD deadlines) |
| 7 | **Children and education** | Age assurance and parental-consent flows (COPPA, ECA Digital, GDPR Art. 8); FERPA/LGPD notes for the LMS starter |
| 8 | **AI transparency** | Disclosure helpers for chatbots and AI-generated content (AI Act), provider data-use notes in `rullst-ai` |
| 9 | **Accessibility** | WCAG 2.2 AA targets for starters, Studio and Nexus, with automated checks in CI |
| 10 | **Data residency** | Region configuration and documentation for storage, mail and AI providers (PIPL and transfer rules) |
| 11 | **Legal page scaffolds** (optional) | Privacy policy, terms and cookie notice templates with explicit placeholders and a prominent "review with counsel" warning |
| 12 | **Framework posture** | CRA position for an open-source project, release security attestations, license and trademark guidance |

## Principles

- **Privacy by default:** starters collect the minimum, never log secrets or
  personal data, and keep optional processing off until enabled.
- **Evidence over claims:** every helper ships tests and documents what it does
  *not* cover, like the rest of Rullst.
- **No hidden legal advice:** templates and guides are starting points with
  clear placeholders, never presented as complete or jurisdiction-proof.
- **Small surface:** features land as optional building blocks so applications
  that do not need them pay nothing.
