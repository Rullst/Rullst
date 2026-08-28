# Security event schema v1

`rullst-security::LiveSecurityEvent` is the bounded event contract rendered by
Studio/Nexus and accepted by the process-local security store. Its current
schema version is exported as `SECURITY_EVENT_SCHEMA_VERSION = 1`.
The package also embeds and exports the JSON Schema 2020-12 document as
`LIVE_SECURITY_EVENT_V1_JSON_SCHEMA`; its packaged source is
[`security-event-v1.schema.json`](../../rullst-security/schema/security-event-v1.schema.json).

This is an application telemetry envelope, not a durable SIEM transport. The
schema does not provide delivery, retention, correlation, acknowledgement,
retry, dead-letter handling, source authentication, or regulatory evidence.

## JSON fields

| Field | v1 contract |
| --- | --- |
| `schema_version` | Integer `1`. Legacy JSON without the field deserializes as v1; locally stored events are normalized to v1. |
| `event_type` | Non-empty uppercase ASCII letters, digits, and underscores; maximum 64 bytes. Invalid values become `SECURITY_EVENT`. |
| `details` | Unstructured human-readable UTF-8 text; maximum 2 KiB and truncated only at a valid character boundary. It must not be parsed as authorization data. |
| `client_ip` | Canonical IPv4/IPv6 string or `unknown`. Forwarded headers are not implicitly trusted. |
| `timestamp_str` | Absolute RFC 3339 timestamp. Invalid local timestamps are replaced at ingestion. |
| `verified_hmac` | `true` only when a connected verifier validated an HMAC for that exact event. It does not prove the event's semantic claim or source identity. `push_local_event` always forces it to `false`. |

Example:

```json
{
  "schema_version": 1,
  "event_type": "RBAC_DENIAL",
  "details": "Authenticated principal denied access to the resource",
  "client_ip": "192.0.2.4",
  "timestamp_str": "2026-08-27T15:30:00.000Z",
  "verified_hmac": false
}
```

## Producer rules

New local producers should use `LiveSecurityEvent::local(...)` and
`SecurityStore::push_local_event(...)`. This path:

1. assigns schema v1 and an RFC 3339 timestamp;
2. validates/bounds the type and detail text;
3. canonicalizes the IP address;
4. removes any caller-provided local HMAC claim; and
5. stores the event in the bounded 50-entry process-local buffer.

The buffer is deliberately a dashboard snapshot. Loss on restart, eviction at
capacity, and absence of a consumer are expected properties, not successful
external delivery.

## Compatibility rule

Within the stable v12 line:

- fields cannot be removed, renamed, or change meaning;
- a new required field or incompatible type requires a new schema version;
- additive optional fields require tolerant consumers and a changelog entry;
- event-type additions are compatible, so consumers need an unknown fallback;
- consumers must not infer trust from `verified_hmac` alone; and
- all compatibility claims apply to JSON, not field order in serialized text.

The version-one contract has source-controlled serialization, legacy-input,
normalization, size, UTF-8, and CEF-injection tests. Release evidence still
belongs to the exact RC tag SHA.

## CEF boundary

`format_cef_event` is a serializer only. It escapes backslashes, equals signs,
and CR/LF in extension values so event details cannot inject fields or records.
Calling `dispatch_siem_alert` records a local SIEM-candidate event; it does not
send or acknowledge an external alert.

A future operational sink should consume the versioned JSON envelope and define
redaction, backpressure, authentication, durable spool/retry/dead-letter,
delivery acknowledgement, retention, and multi-tenant access separately.
