### Fixed

- `cargo rullst upgrade` reported `V13-CSRF-STATIC-PREFIXES` for routes and
  file services that the project's `Rullst.toml` already exempts: it compared
  their paths with Core's default `csrf_static_prefixes` and
  `csrf_static_paths`, not with the `[security]` lists `Server` reads, so a
  project that had followed the rule's own advice got a finding for each
  route below its `/assets/` prefix or listed in `csrf_static_paths`. The
  rule now reads both lists from `Rullst.toml` as `Server` does (a missing
  key keeps its default, a list replaces it) and leaves out the paths they
  cover, a `nest_service` only below a prefix (an exact path, even the
  mount point, matches none of its files). Paths are compared as written,
  without the prefix of an enclosing `.nest(..)`. Without a `Rullst.toml`
  it uses the defaults, as before; a `Rullst.toml` that `Server` refuses to
  load (it does not parse, or its `[security]` settings are invalid) keeps
  the defaults, and each finding then says why. Its message no longer says that the router serves files
  when the route's handler renders the response (#895).
