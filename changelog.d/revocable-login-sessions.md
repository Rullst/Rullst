### Added

- Server-side login sessions for `rullst_session` cookies:
  `rullst::auth::session_registry` (features `session-registry-sqlite`,
  `session-registry-postgres` and `session-registry-mysql` of `rullst-auth`;
  `auth-session-registry-sqlite`, `-postgres` and `-mysql` of `rullst`).
  `SessionRegistry` keeps one row per sign-in in the table
  `rullst_auth_sessions` of the application's own SQLx pool, created on first
  use, and issues `v3.` cookies that carry a random 256-bit session secret
  besides the account, its session epoch and the expiry; the row stores only
  a SHA-256 digest of the secret, so a copy of the database yields no cookie.
  Every request checks the row by primary key, without a cache. Sign-out
  (`sign_out`, `revoke`) ends every copy of the cookie, which a stateless
  cookie could not; `list_sessions` shows an account's sessions (sign-in
  time, hour of last use, browser and platform family, no IP address or raw
  `User-Agent`), and `revoke_session`, `revoke_other_sessions` and
  `revoke_all_sessions` sign out one, the others or all of them. An account
  keeps at most 50 sessions, the least recently used one making room, and
  `purge_expired` deletes expired rows in batches of up to 1,000.
  `decrypt_session_claims` refuses `v3.` cookies, so a route that skips the
  registry fails closed; the session epoch keeps working. Applications that
  do not enable a registry feature are unchanged.
- `cargo rullst auth` and the SaaS starter use the registry: registration and
  sign-in register a session, `/logout` deletes it, the new "Your devices"
  page (`/sessions`, `src/controllers/sessions_controller.rs`) lists the
  account's sessions and signs out one of them or every other one, a password
  change deletes the account's sessions and gives the asking browser a new
  one, and a password reset and "Sign out everywhere" delete them all.
  `make:mfa` and `make:oauth` register their sign-ins the same way in such a
  starter; in a starter generated before 13.0.0-alpha.6 they keep issuing
  session-epoch cookies.
