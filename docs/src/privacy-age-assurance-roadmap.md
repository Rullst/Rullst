# Privacy and age assurance roadmap

**Status: v13 P0, initial unpublished age-assurance foundation, reviewed
17 September 2026.** The [crate](../../rullst-privacy/README.md) implements
risk policies, bound challenges, signed threshold attestations, explicit
outcomes and replay-store contracts. Broader privacy workflows, concrete live
age providers and production replay storage remain unimplemented.

Rullst should generate applications with privacy-preserving defaults and
reusable, testable controls. It must not advertise automatic worldwide legal
compliance. The applicable rules depend on the operator, audience, processing
purpose, territory and service; organizational duties cannot be inferred from
a Cargo feature. This is an engineering plan requiring jurisdiction-specific
legal review before a profile is presented as supported.

## Existing foundations and the missing boundary

The ORM already offers field encryption. Auth/Security provide identity,
authorization and redaction primitives. The LMS privacy templates already
record school-scoped policies, nominal `Adult`/`Minor` bands, guardian-consent
records and bounded export/delete request processing. They do not independently
verify a person's age or a guardian's authority. The privacy worker still
requires an application adapter to perform actual export/deletion.

These pieces should be reused, with an explicit migration for generated LMS
applications. Adding a consent row, encrypting a field or completing a mock job
does not establish lawful processing or prove that downstream copies were
erased.

## Package decision

Use the optional **`rullst-privacy`** crate with its **`age-assurance`** feature
and **`age_assurance`** module. Age assurance belongs with purpose, minimization, retention and child
protection policy; Auth consumes its evidence when deciding access. A separate
age-only crate is unnecessary for the first bounded contract.

| Component | Proposed responsibility |
| :--- | :--- |
| `rullst-privacy` | Versioned purposes/policies, lawful-basis metadata, consent and withdrawal, rights-request orchestration, retention/hold contracts, guardian evidence and age-assurance policy/receipts. Static-dispatch adapters and typed errors. |
| `rullst-auth` | Bind evidence to the authenticated session/subject and enforce the requested age-restricted action server-side. An age receipt never replaces authentication, tenant scope or ordinary authorization. |
| `rullst-security` | Reuse encryption, abuse controls, signature primitives and minimized audit. Do not duplicate cryptography or bury the privacy domain inside WAF middleware. |
| `rullst-orm` and Core | Optional persistence and bounded jobs. Application adapters enumerate actual stores; no reflection-based discovery or automatic traversal of arbitrary data. |
| `rullst-connect` | Consume a reviewed issuer's identity/age claims only when their semantics are established; an OAuth login alone proves no age. |
| `cargo-rullst`, Nexus and Studio | Generate explicit application configuration, accessible preferences/rights flows and authorized, minimized operational views. Never display selfies or identity documents as telemetry. |
| `rullst-ai` | Optional explanations with approved, minimized context. An LLM's visual guess must not authorize age-restricted access or establish guardianship. |

The base crate must not require a database, camera, remote provider, inference
runtime or new default Core dependency. Provider/persistence integrations stay
feature-gated. The initial package is a workspace member with `publish = false`
and no umbrella/Core dependency; keep it outside the release-order manifest
until the v13 package and provider/state acceptance gates pass.
A future local vision engine may need its own package/model lifecycle;
that is separate from this first policy and adapter contract.

## Global policy coverage

Profiles are versioned implementation targets, not legal certificates. Each
needs authoritative references, applicability assumptions, effective dates,
last-review date, reviewer, tested controls and explicit gaps. Do not select
law solely from language, IP geolocation or a country dropdown. More than one
regime may apply; unresolved requirements need an operator decision, not an
automatic “strictest country wins” rule.

| Wave | Jurisdiction and review scope | Initial source |
| :--- | :--- | :--- |
| First | Brazil: LGPD and child-protection/ECA Digital requirements | [LGPD](https://planalto.gov.br/ccivil_03/_ato2015-2018/2018/lei/l13709.htm), [ANPD age-assurance guidance programme](https://www.gov.br/anpd/pt-br/assuntos/noticias/anpd-tomada-de-subsidios-afericao-idade) |
| First | EU/EEA: GDPR, country-specific child-consent ages, applicable ePrivacy and DSA duties | [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng), [EDPB age assurance](https://www.edpb.europa.eu/documents/statement/statement-12025-on-age-assurance_en) |
| First | UK: UK GDPR/DPA, PECR, Children's Code and applicable Online Safety Act duties | [ICO Children's Code](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/childrens-information/childrens-code-guidance-and-resources/), [Ofcom age assurance](https://www.ofcom.org.uk/online-safety/protecting-children/age-assurance) |
| First | US: CCPA as amended by CPRA where applicable, COPPA, plus a maintained state/biometric-law applicability inventory | [California DOJ](https://www.oag.ca.gov/privacy/ccpa), [FTC COPPA amendments](https://www.ftc.gov/legal-library/browse/federal-register-notices/16-cfr-part-312-coppa-final-rule-amendments) |
| Expansion | Canada: PIPEDA and provincial regimes, including Quebec; Australia: Privacy Act/APPs and applicable child-safety rules | [Canadian OPC](https://www.priv.gc.ca/en/privacy-topics/privacy-laws-in-canada/), [Australian OAIC](https://www.oaic.gov.au/privacy/australian-privacy-principles) |
| Expansion | Japan: APPI; India: DPDP Act/rules with phased commencement tracked explicitly | [Japanese PPC](https://www.ppc.go.jp/en/legal/), [India's rules and commencement documents](https://www.meity.gov.in/documents/act-and-policies/digital-personal-data-protection-rules-2025-gDOxUjMtQWa?pageTitle=Digital-Personal-Data-Protection-Rules-2025) |

Additional named review targets are Switzerland's FADP, South Africa's POPIA,
Singapore's PDPA, South Korea's PIPA and China's PIPL, followed by the countries
actually served in Latin America and the Middle East. Their detailed mappings
are not reviewed or supported by this document. A “global” label must never
hide missing jurisdictions, sector rules, localization or cross-border duties.

Rules also evolve within a release cycle. The ANPD opened consultation on age
assurance guidance in May 2026; a draft must not be treated as a final technical
mandate. The FTC's February 2026 age-verification enforcement policy has
conditions and is not a general exemption from COPPA. Profiles need legal-update
review separate from framework version updates.
[ANPD consultation](https://www.gov.br/anpd/pt-br/assuntos/noticias/anpd-tomada-de-subsidios-afericao-idade),
[FTC policy statement](https://www.ftc.gov/news-events/news/press-releases/2026/02/ftc-issues-coppa-policy-statement-incentivize-use-age-verification-technologies-protect-children).

## Controls that can be automated

1. **Purpose-bound processing:** explicit data-category and purpose inventory,
   legal-basis metadata, processor destinations and retention rules. Consent is
   one possible basis, not a substitute for every basis or a blanket terms checkbox.
2. **Conservative defaults:** unnecessary tracking, advertising, remote embeds
   and personal-data collection disabled. Essential session/CSRF controls stay
   distinct from optional analytics. Only activate integrations allowed by the
   reviewed policy and current preferences.
3. **Consent and choices:** purpose-specific, versioned receipts; equally usable
   refusal/withdrawal; server-side enforcement; age-appropriate notices. Treat
   GPC as the relevant sale/sharing opt-out signal under an applicable profile,
   not as a universal consent value. California describes this obligation for
   covered businesses in its [CCPA guidance](https://www.oag.ca.gov/privacy/ccpa).
4. **Rights workflows:** authenticated access/export/correction/delete requests,
   applicable objection/restriction/opt-out paths, representative authority,
   bounded deadlines, retries and delivery evidence. Verify identity
   proportionately and prevent cross-user/tenant exports. Minimize any retained
   verification data.
5. **Retention and erasure:** explicit adapters for SQL, files, caches, search and
   vector indexes, queues, mail, AI history and processors. Coordinate racing
   jobs so deleted data is not recreated. Legal holds have bounded purpose,
   restricted access and review; soft deletion is not erasure. Track backup
   expiry and reapply erasure records on restore. Pseudonymization is not a
   claim of irreversible anonymization.
6. **Evidence and operations:** minimized audit, data-flow/processor inventory,
   DPIA/RIPD inputs, incident workflow and policy-version reports. Export a
   control assessment with “unconfigured/unsupported” states, never a universal
   “legally compliant” badge.

The operator still owns applicable-law decisions, lawful purpose, notices,
contracts, transfer mechanisms, DPO/representative duties where required,
responses to people/regulators and incident handling. The framework can record
and enforce configured decisions; it cannot make those decisions correct.

## Age assurance contract

Keep three policies separate: eligibility for a particular service/action,
capacity to consent to particular processing, and verified guardian authority.
There is no universal 18-year threshold for every application or consent flow.
GDPR Article 8 concerns a particular consent-based information-society-service
context and allows national thresholds within its bounds; it is not a universal
account-registration age. [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng).

An Academy may support minors with appropriate protections. Its risk assessment
must determine when stronger evidence is needed; a camera check must not become
mandatory merely to read a lesson. Guardian consent cannot override a legally
prohibited activity. A recorded guardian relationship needs its own evidence,
scope, expiry and withdrawal contract.

| Method | Meaning and planned treatment |
| :--- | :--- |
| Self-declared age/band | Low-assurance declaration, clearly labeled. Never silently promote it to verified evidence or use it where the policy requires a stronger method. |
| Facial age estimation | Probabilistic estimate from a specialized provider or evaluated local model; not exact age or identity. Optional adapter, subject to the gates below. |
| Verified age attribute | A reviewed digital credential, provider/document check or other appropriate method returning the minimum threshold/band needed. Authenticity and subject binding must be verified. |

Facial estimation is not inherently useless: Ofcom lists it among methods
capable of being highly effective, subject to technical accuracy, robustness,
reliability and fairness. A photo upload alone does not establish these
properties. NIST's evaluation reports demographic and image-quality effects.
[Ofcom guidance](https://www.ofcom.org.uk/online-safety/illegal-and-harmful-content/online-pornography),
[NIST evaluation](https://pages.nist.gov/frvt/html/frvt_age_estimation.html).

The planned contract must:

- distinguish passed, below-threshold, inconclusive, unavailable, expired and
  offline-mock results; inconclusive/unavailable cannot grant restricted access;
- offer an accessible alternative and appeal/review flow, without treating a
  provider outage or lack of camera as permission to bypass the required check;
- use a validated challenge-age margin near the threshold and an alternative
  method for uncertain cases; a raw model confidence score is not a calibrated
  probability or independently verified accuracy;
- test presentation/replay attacks, borrowed photos, synthetic media and
  capture injection; liveness is one control and needs evidence of its own;
- bind issuer, audience, session/subject, tenant, policy version, threshold,
  method, expiry and a one-use challenge to authenticated provider evidence;
  verify signatures/callbacks, prevent cross-context replay and define key
  rotation/revocation. Browser-authored booleans or scores are never trusted;
- retain only the policy result and necessary bounded provenance. Prefer a
  threshold proof over exact birth date, document number or a reusable face
  identifier; use pairwise references to limit cross-service correlation;
- avoid retaining raw selfies/documents or their reusable hashes in application
  storage, logs, telemetry, queues or backups. Prefer direct provider capture
  under an explicit data-processing contract or suitable on-device processing;
  any transient application handling needs strict size/time/access limits;
- require provider purpose limits, deletion evidence and no secondary training,
  advertising or identity profiling. Not storing a photo locally does not mean
  no personal data was processed;
- retain deterministic empty/`mock_*` provider behavior for development, marked
  with mock provenance that production authorization must reject.

Under GDPR, a photograph is not automatically Article 9 biometric processing;
the processing purpose, including unique identification, matters. Other laws
may classify facial data differently. Both the image and derived result need a
reviewed processing basis and safeguards; “age estimation only” is not a blanket
exemption. [GDPR, recital 51 and Article 9](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng).

## Blueprints, examples and Academy

All generated application shapes should share the reviewed privacy defaults.
Generate an operator-completed data inventory and configuration, notices that
identify missing information, and accessible preference/rights routes where
the selected blueprint processes personal data. Do not insert camera flows,
cookie banners or a heavyweight provider into a database-free starter that has
no corresponding need.

SaaS must distinguish billing records retained for a reviewed obligation from
marketing data that can be removed. LMS/Academy must protect learner progress,
guardian relationships, tutor history, submissions and any optional exam
supervision separately. Age assurance does not authorize surveillance or prove
exam misconduct. API-only projects need equivalent server-side enforcement.

`Rullst/examples` and the private Academy remain separately released consumers.
Use synthetic test subjects and publish only sanitized framework evidence.
Record application/framework versions and provider environments. A generated
mock flow is not deployment evidence for either production or staging.

## Prioritized delivery and acceptance

| Phase | Delivery | Required evidence |
| :--- | :--- | :--- |
| **P0.0** | Threat model, jurisdiction/profile format, data flow, package ADR and LMS migration design | Reviewed applicability assumptions, unknown/conflict behavior, model/provider lifecycle and first Academy/SaaS journeys. |
| **P0.1** | Bounded privacy policy, consent/withdrawal, rights/retention contracts and explicit age/guardian evidence types | Deterministic tests for withdrawal, purpose changes, expired evidence, replay, tenant isolation and production mock rejection. |
| **P0.2** | First real privacy workflow and provider-neutral age gate; one reviewed external age adapter with a non-facial alternative | Protocol and authorized sandbox evidence, session binding, outage/cancellation handling, deletion and appeal flow. No production age claim from mocks. |
| **P0.3** | SaaS/LMS blueprint adoption and versioned examples/Academy migration | Browser/API negatives, no optional tracking before permission, real export/delete execution, backup-restore behavior and no image/identity leakage. |
| **After foundation** | More regional profiles, credential issuers and an optional local facial estimator | Per-profile review; model provenance/licensing, integrity-pinned weights, supported devices, threshold error rates, demographic fairness and attack evaluations. No bundled unvalidated “simple AI”. |

Facial estimation is in the v13 priority scope as an optional, evaluated adapter;
shipping a Rullst-trained model is not a prerequisite. Release claims require
false-accept/false-reject results near each chosen threshold, relevant audience
and device coverage, data lifecycle evidence, accessibility and independent
review appropriate to the risk. One provider's evaluation does not validate
every adapter or jurisdiction.

The first privacy/age journey takes precedence over additional learning-game,
monitoring and speculative integration features. Existing v12.1 maintenance and
verification-efficiency work remain separately deliverable. See the
[SaaS maintenance triage](saas-v12-1-v13-triage.md) for the payment work already
required before enabling affected live operations.

The planned [v13 Verus pilot](verus-roadmap.md) starts with this crate's method,
threshold and evidence-decision contracts. Its proofs must follow the actual
production functions and state their clock, issuer and replay-store assumptions.
They supplement the acceptance evidence above; facial accuracy, guardianship,
live provider behavior and legal applicability require their own review.
