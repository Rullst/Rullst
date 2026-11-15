### Added

- Server-side login sessions for `rullst_session` cookies:
  `rullst::auth::session_registry` (features `session-registry-sqlite`,
  `session-registry-postgres` and `session-registry-mysql` of `rullst-auth`;
  `auth-session-registry-sqlite`, `-postgres` and `-mysql` of `rullst`).
  `SessionRegistry` keeps one row per sign-in in the table
  `rullst_auth_sessions` of the application's own SQLx pool, which a
  migration creates with `create_table` (the application then uses
  `connect`, which creates nothing) or `open` creates on first use, and
  issues `v3.` cookies that carry a random 256-bit session secret
  besides the account, its session epoch and the expiry; the row stores only
  a SHA-256 digest of the secret, so a copy of the database yields no cookie.
  Every request checks the row by primary key, without a cache. Sign-out
  (`sign_out`, `revoke`) ends every copy of the cookie, which a stateless
  cookie could not; `list_sessions` shows an account's sessions (sign-in
  time, hour of last use, browser and platform family, no IP address or raw
  `User-Agent`), and `revoke_session`, `revoke_other_sessions` and
  `revoke_all_sessions` sign out one, the others or all of them
  (`revoke_all_sessions_on` inside the caller's transaction). An account
  keeps at most 50 sessions, the least recently used one making room, and
  `purge_expired` deletes expired rows in batches of up to 1,000.
  `decrypt_session_claims` refuses `v3.` cookies, so a route that skips the
  registry fails closed; the session epoch keeps working. A request is
  checked when it starts, so one already running when its session is revoked
  completes. `revalidation(&current)` is the check that ends a Server-Sent
  Events stream or WebSocket of the session: at once when a registry of the
  same process revokes it, and within one revalidation interval when another
  process does. Applications that do not enable a registry feature are
  unchanged.
- Long-lived responses can be revalidated while they stay open:
  `RealtimeSse::channel_revalidated` and `subscribe_revalidated`, and
  `WebSocket::with_revalidation`, take a `rullst::realtime::Revalidate`
  check (or `revalidate_with` of an async closure) that runs every
  revalidation interval (15 seconds by default,
  `RealtimeSseConfig::with_revalidation_interval`, 1 second to 5 minutes)
  and when the check signals a change, with a 5-second deadline; the stream
  or socket ends when it fails. `WebSocket` gained a type parameter for its
  check, which defaults to `NoRevalidation`, so `WebSocket::new` and existing
  uses are unchanged.
- `cargo rullst auth` and the SaaS starter use the registry: a
  `*_create_rullst_auth_sessions_table` migration owns its table (rolling it
  back drops it), registration and
  sign-in register a session, `/logout` deletes it, the new "Your devices"
  page (`/sessions`, `src/controllers/sessions_controller.rs`) lists the
  account's sessions and signs out one of them or every other one, a password
  change deletes the account's sessions and gives the asking browser a new
  one, and a password reset and "Sign out everywhere" delete them all.
  `make:mfa` and `make:oauth` register their sign-ins the same way in such a
  starter, and `make:oauth`'s `LinkAndResetPassword` policy deletes the
  account's sessions in the transaction that removes its password; in a
  starter generated before 13.0.0-alpha.6 they keep issuing session-epoch
  cookies. `auth_middleware::revalidation(&session)` is the check for a
  Server-Sent Events stream or WebSocket of the session.
