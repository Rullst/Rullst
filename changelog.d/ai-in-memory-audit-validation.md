### Changed

- `rullst-ai`'s `InMemoryToolAuditTrail` now validates each event as
  `DurableToolAuditTrail` does. An event with an empty or oversized principal,
  an invalid tool identifier, incomplete or oversized approval evidence, or no
  risk outside a denial is not recorded, and `record` returns
  `ToolExecutionError::AuditUnavailable` with the durable trail's message.
  Events built by `ToolRegistry::execute` always pass; only an event that an
  application builds or deserializes itself can be refused.
