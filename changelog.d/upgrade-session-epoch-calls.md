### Fixed

- `V13-AUTH-SESSION-REGISTRY` only inspected the five file names the
  generators wrote and went silent once a manifest enabled a session
  registry, although the registry's middleware refuses the session-epoch
  cookies that `decrypt_session_claims` reads and `make_login_cookie_with_epoch`
  issues: such code compiles and passes the tests that need no database, but
  fails at run time (the `cookie_epoch` of the `password_controller.rs` that
  `cargo rullst auth` generated before alpha.6 sends every password change
  back to sign-in without changing the password). The rule now reports each
  of those calls, not `use` lines, in any file outside tests, and the new
  review rule `V13-AUTH-SESSION-EPOCH-CALLS` reports each call that remains
  once a manifest enables `auth-session-registry-*` (or `rullst-auth`'s
  `session-registry-*`). Step 16 of the alpha.5 to alpha.6 checklist names the
  old generated `cookie_epoch` (#825).
