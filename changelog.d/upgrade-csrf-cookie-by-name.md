### Fixed

- `V13-CSRF-COOKIE-IN-PAGE` recognised only the `.strip_prefix("rullst_csrf=")`
  of the `get_csrf_token` helper that the generators wrote before
  13.0.0-alpha.6, so an application's own helper that read the cookie with
  `cookie(headers, "rullst_csrf")` and put the raw value in a page went
  unreported. The rule now reports every `"rullst_csrf"` or `"rullst_csrf="`
  string outside test code in a file that never calls `mask_csrf_token`
  (`jar.get("rullst_csrf")`, `c.name() == "rullst_csrf"`, a constant holding
  the name); a string that only contains the name, such as the
  `format!("rullst_csrf={token}")` of a request that sends the cookie, and a
  file that masks the value are not reported (#824).
