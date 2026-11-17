### Fixed

- On Windows, the durable AI audit trails (`DurableRagAuditTrail`,
  `DurableToolAuditTrail`) and both SIEM spools (`DurableSiemSpool`,
  `AuthenticatedSiemSpool`) could not roll back a partial write, for example
  on a full disk. They opened their file in append mode, and a Windows append
  handle has no write-data access, so truncating the partial frame always
  failed: the writer reported `RecoveryFailed`, became unhealthy, and the
  partial frame stayed in the file. On Windows they now open the file for
  reading and writing and seek to its end before each frame, so a failed
  write is rolled back as on Unix and the writer stays usable. The Unix open
  flags and O_APPEND writes are unchanged, as is the single-writer contract:
  one writer per file, and growth by anything else fails closed as an
  external modification.
