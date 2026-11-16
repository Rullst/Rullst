### Changed

- **Breaking:** for the 13.0 public API freeze, every publicly reachable enum,
  and every struct whose fields are all public, is now either
  `#[non_exhaustive]` or documented as exhaustive on purpose, so a 13.x
  release can add a variant or a field without breaking applications. 39
  enums (the errors, statuses, kinds, modes and backends that can grow, such
  as `StorageError`, `SecurityError`, `SubscriptionStatus`, `ModelOperation`,
  `ColumnDefault`, `AgeError`, `JwtPolicyMode` and IoT's `NodeStatus`) and 103
  structs (among them `FieldMeta`, `RegistryEntry`, Mail's `Message`,
  `ConnectUser`, `ExchangeParams`, `AuthCallback`, `HttpResponse`, `Passkey`,
  `WebhookEvent`, `QueuedJob`, `TraceSpan`, `ToolParam`, `PaginationResult`
  and `TimingGuardConfig`) are `#[non_exhaustive]`. A `match` on one of these
  enums outside its crate needs a wildcard arm, and a struct literal of one
  of these structs, `..Default::default()` included, no longer compiles
  outside its crate: use the constructors, several of them new
  (`FieldMeta::new(..).hidden().readonly()`,
  `ExchangeParams::new(code).with_code_verifier(..)`, `ConnectUser::new`,
  `HttpRequest::new`, `HttpResponse::new`, `TokenExchangeForm::new`,
  `Passkey::new`, `WebhookEvent::new(..).with_ends_at(..)`,
  `WebhookEventRecord::new`, `RevenueMetrics::new`, `QueuedJob::new`,
  `QueuedJobDetail::new(..).with_error(..)`, `TraceSpan::new`,
  `ToolParam::new`, AI's `Message::new`, `RagRetrievalError::new`,
  `RagAuditError::new`, `PaginationResult::new`, `PrivacyReport::new`,
  `RegistryEntry::new`, `NexusState::new` and `TimingGuardConfig::new`), or
  start `AuthCallback` and `ClaimedOutboxEvent` from `default()` and
  `ProviderCapabilities` from `PORTABLE` or `NONE`. Reading and assigning
  public fields keeps working. `#[derive(Orm)]`, `#[derive(PersonalData)]`
  and `#[derive(Nexus)]` generate constructor calls, and the SaaS starter's
  Nexus models and the `make:oauth apple` controller no longer write
  literals; projects generated before alpha.6 have such literals to rewrite.
  The authorization and decision enums, closed sets and versioned wire
  formats stay exhaustive on purpose, so a new variant there breaks callers
  until they handle it: Labs' `Action`, `Teardown`, `LeaseStatus` and
  protocol v1 structs, Privacy's `AgeDecision`, `AgeOutcome`, `Assurance` and
  `AgeDeclaration` and the `AgeMethod` and `RiskLevel` inputs of the
  Verus-proven `AgePolicy::permits`, Auth's `CeremonyKind`, Core's `AttributePresence`,
  `CompletionStatus` and `AbortStatus`, IoT's `MqttQos` and `PinState`,
  Studio's `DistributedTraceStatus`, `TraceBatchV1` and
  `DistributedTraceSpanV1`, and the newtypes `ValidatedJson`,
  `ValidatedForm`, `Inject`, `RawHtml` and the ORM's `Json`.
  `cargo rullst upgrade` reports each `match` without a wildcard arm
  (`V13-NON-EXHAUSTIVE-MATCH`) and each literal
  (`V13-NON-EXHAUSTIVE-LITERAL`), and the public API snapshot check now
  fails on a new public enum or all-public struct that is neither
  `#[non_exhaustive]` nor documented as exhaustive on purpose.
