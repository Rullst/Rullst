### Changed

- **Breaking:** before the 13.0 freeze, three pre-1.0 crates that break every
  few months left the public API, so that 13.x releases can update them as
  private dependencies; the public dependencies of 13.x go from 28 to 25.
  `DistributedTelemetry::layer` returns
  `impl tracing_subscriber::Layer<S> + Send + Sync + 'static` instead of
  `tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::SdkTracer>`,
  so `opentelemetry_sdk` and `tracing-opentelemetry` are private. Code that
  wrote that type writes `impl Layer<S>` or boxes the layer, and calls of
  `OpenTelemetryLayer`'s own builder methods on it (`with_location`,
  `with_threads`, …) are dropped: the minimized pipeline removes the
  attributes and events they add. `registry().with(runtime.layer())`,
  `.with_filter(..)` and `.boxed()` keep working, the subscriber type must be
  `Send + Sync`, and the exported spans and `TraceParent` are unchanged.
- **Breaking:** `ValidationError::ValidationError` carries the new
  `rullst::FieldErrors` instead of `validator::ValidationErrors`.
  `messages()` and `codes()` return each field path's messages and validator
  codes, `is_empty()` whether any failed, and `as_any()` reaches the
  `validator` errors by downcasting; `FieldErrors::validate(&payload)`,
  `FieldErrors::default()` and `add(field, code, message)` build one, so a
  handler can add a failure such as a taken email address to the same `422`
  response. The JSON response, the HTMX fragment, `Display` and `Debug` are
  unchanged. `validator` stays a public dependency through `rullst::Validate`
  and the bound of `ValidatedForm` and `ValidatedJson`.
- `Studio::with_openapi` and `api_playground::router` take any
  `serde::Serialize` document instead of utoipa 6's `OpenApi`: the `OpenApi`
  of any utoipa version, or a `serde_json::Value`, so `utoipa` is private and
  an application on utoipa 5 can use Studio again.
  `.with_openapi(ApiDoc::openapi())` compiles as before; an argument whose
  type came from the parameter (`Default::default()`, an untyped
  `serde_json::from_str`) now names it. A utoipa document renders exactly as
  before; another OpenAPI 3.1 or 3.2 document is listed too, and for any
  other document the page says it cannot list the operations and links the
  JSON, which `/studio/api/openapi.json` serves as given.
- `cargo rullst upgrade` reports these with the must-change rules
  `V13-TELEMETRY-LAYER-TYPE`, `V13-VALIDATION-FIELD-ERRORS` and
  `V13-STUDIO-OPENAPI-INFERRED`, and no longer reports
  `V13-STUDIO-UTOIPA-6`, which asked a workspace on utoipa 5 to move to 6.
- The compatibility policy lets a 13.x minor release support a newer,
  incompatible major of a public dependency behind a new opt-in feature,
  side by side with the 13.x line when both can be compiled into one
  application; the default stays on the 13.x line. It also records why
  `tower-sessions` stays public: Connect's `axum-session` extractors read
  the `Session` that the application's own `SessionManagerLayer` inserts.
