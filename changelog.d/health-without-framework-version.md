### Changed

- **Breaking:** `/health` and `/ready` no longer report the framework version.
  Every `rullst::health` router answered `"version":"13.0.0-alpha.5"` to any
  visitor, which tells a scanner which known vulnerabilities to try.
  `health_router()` and `health_router_with_lifecycle` now answer `status`,
  `uptime_seconds` and `timestamp` (with the lifecycle counts on `/ready`),
  and `health_router_with_version(app_version)` and
  `health_router_with_lifecycle_and_version` add only the application's
  `app_version`. The framework version stays on the authenticated production
  monitor (`framework.version` of `GET /_rullst/monitor/v1`).
  `HealthProbeResponse` and `ReadinessProbeResponse` lose their `version`
  field, and `HealthProbeResponse` is `#[non_exhaustive]`. The CLI's
  generated deploy, Kubernetes and container checks read only the status
  code and are unchanged. `cargo rullst upgrade` reports code and scripts
  that read `version` from a probe (`V13-HEALTH-FRAMEWORK-VERSION`) and uses
  of the removed field (`V13-HEALTH-PROBE-VERSION-FIELD`).
