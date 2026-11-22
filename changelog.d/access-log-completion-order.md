### Fixed

- The telemetry guide, the specification and the alpha.5 to alpha.6
  checklist now say that access-log lines are written in the order in which
  responses finished, not in which requests arrived, and that a line carries
  no arrival time or request id, so a script that reads the log as a
  sequence of requests matches lines by path and status, or uses its own
  request ids or the OpenTelemetry spans (#828).
