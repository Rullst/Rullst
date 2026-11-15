### Fixed

- **Turso/libSQL:** saving a model whose text primary key a database
  `DEFAULT` fills now works on a multi-instance server (such as Turso Cloud)
  that answers the insert's interactive Hrana stream with a `base_url`: the
  commit or rollback goes to that URL when it has the configured endpoint's
  scheme, host and port. A `base_url` at another origin is refused with the
  new `PolyglotError::CrossOriginEndpoint` before anything, the auth token
  included, is sent there; the insert is not committed. Before, the client
  ignored `base_url` and such an insert failed without writing a row.
