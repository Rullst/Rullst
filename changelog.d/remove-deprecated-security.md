### Removed

- **Breaking:** `rullst-security`'s `RateLimiter::with_distributed` and
  `AuditChain::new`, deprecated in 12.0.0, were removed. `with_distributed`
  selected a distributed backend that does not exist, so every request was
  refused; `try_with_distributed` still reports that as
  `RateLimitError::DistributedBackendUnsupported` at start-up, and the
  `redis-rate-limit` feature's `RedisRateLimiter` shares a budget between
  processes. Replace `AuditChain::new(key, logger)` with
  `AuditChain::try_new(key, logger)?`, which rejects a weak key (shorter than
  32 bytes or with fewer than eight distinct byte values) at start-up instead
  of building a chain that refuses every event. `cargo rullst upgrade`
  reports both (`V13-SECURITY-DEPRECATED-REMOVED`).
