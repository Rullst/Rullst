### Changed

- The security baseline's header step (also `headers_middleware`) now
  appends its headers without looking each one up first when a response has
  none of them and at most eight header names, the usual case, instead of
  about fifteen map lookups per response. Every response keeps the same
  headers with the same values in the same order; a response that sets one
  of them itself takes the previous path.
