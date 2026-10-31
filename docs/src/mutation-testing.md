# Mutation Testing Record

This page records what the mutation campaigns have measured and which
surviving mutants are accepted as equivalent. The workflows are described in
[WORKFLOWS.md](../../WORKFLOWS.md#pull-request-mutation-gate): `mutants.yml`
runs full, targeted, contract and recovery campaigns, and `mutants-pr.yml`
mutates the changed lines of every pull request to `main`. How to run a
campaign and kill survivors is in
[MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#6-mutation-campaigns).

## Campaign status

Measured on hosted runners for the 13.0 prereleases. A missed mutant either
received a test or is listed under
[Accepted equivalent mutants](#accepted-equivalent-mutants).

- **`rullst-orm`** (re-measured for 13.0.0-alpha.3 with all-feature targeted
  runs and the `orm-sqlite`, `orm-polyglot`, `orm-sql-servers`,
  `orm-strict-postgres`, `orm-strict-mysql` and `orm-polyglot-servers`
  contract modes): 2,414 mutants, 1,967 caught, 421 unviable, 17 timeouts and
  9 missed. Eight are accepted equivalents. The ninth is the MySQL
  `FOR UPDATE SKIP LOCKED` claim query of `Outbox::claim_next_at_inner`, which
  only concurrent claims tell apart from the portable query. A full
  all-feature campaign still reports the mutants that only those contract
  modes reach.
- **`rullst-orm-macros`**: 542 mutants, 514 caught, 26 unviable and 2 accepted
  equivalents.
- **`rullst-core`**: the files the first pass left
  ([PR #465](https://github.com/Rullst/Rullst/pull/465)) were finished in
  [PR #532](https://github.com/Rullst/Rullst/pull/532): 78 files and 2,435
  mutants, 72 missed at the end. 71 are accepted equivalents; the last one
  needs an open PostgreSQL pool.
- **`rullst-nexus`**: first measured in PR #532 (26 files, 876 mutants). New
  tests killed 170 of the 171 first misses, one is an accepted equivalent, and
  one real defect was fixed.
- **`cargo-rullst`** ([PR #557](https://github.com/Rullst/Rullst/pull/557),
  13.0.0-alpha.4): targeted runs with the CLI's library tests measured 83 CLI
  files and 3,189 mutants of the self-update, audit and IDOR scanner, `add`
  and consumer-file editors, `doctor`, deployment configuration and `upgrade`
  code. New tests killed 1,041 of the 1,129 first-run survivors and 51 are
  accepted equivalents. The 76 files left without a survivor are listed in
  `.github/mutants-pr-cli.txt`. The self-updater's binary replacement was
  removed in 13.0.0-alpha.4 with its 7 remaining installation mutants, and its
  13 measured files left the gate list with it. Still missed: size and count
  bounds that only very large projects reach (hundreds of mebibytes, or 10,000
  to 250,000 files): upgrade snapshot 6, upgrade source walk 3, project
  snapshot 13, replacement and verification totals 2 and 2, and the 64 MiB
  preparation report 4.
- **`green`, `deploy` and guided update** (13.0.0-alpha.4): the nine `green`
  files, `generators/deploy.rs` and `update/guided.rs` gave 337 mutants, 93
  missed and no timeout. New library tests (a loopback test server, fake app
  and container-engine scripts, child test processes for commands that use
  the working directory or the terminal) killed 86; the other 7 are accepted
  equivalents. The 11 files are in `.github/mutants-pr-cli.txt`.
- **Studio remote mode** (20 files, measured with the `cargo-rullst` campaign):
  no survivor left.
- **`rullst-capital`**: 29 mutants that only a live Stripe response reaches
  stay with provider sandbox testing.
- **v12 baseline**: the complete v12 campaign classified 14,391 of 14,391
  mutants in run `34761010296`, with a conservative 70.57% caught.

## Accepted equivalent mutants

The v13 focused campaign (targeted runs of security and correctness files
before 13.0.0-alpha.1), the local whole-crate campaign that followed it
(`rullst-iot`, `rullst-privacy`, `rullst-labs`, `rullst-supervision`), the
security campaign (targeted runs of every `rullst-security` file), the
`rullst-auth` campaign (targeted runs of every file outside the PostgreSQL
adapters), the `rullst-core` campaign (targeted runs of its security and
correctness files and, before 13.0.0-alpha.3, of every remaining file the
first pass measured, with the Redis service), the `rullst-nexus` campaign
(targeted runs of every file; the all-feature build selects the PostgreSQL
pool, so the PostgreSQL SQL matrix test reaches the database paths wherever
Docker is available),
the `rullst-capital` campaign (targeted runs of every file; mutants that
only a live Stripe API response can reach are left to provider sandbox
testing), the `rullst-messaging` campaign (targeted runs of every file,
with the recurring PostgreSQL and Redis Streams contract runs for those
adapters), the service-backed campaign (the `passkey-postgres`,
`auth-recovery-postgres`, `privacy-postgres` and `mail-postgres` contract
runs, and targeted runs of the Redis rate limiter with the Redis service) and
the sensitive-file campaign (targeted runs of the OAuth/OIDC, attachment,
link-security, tracking, egress and guardrail files of `rullst-connect`,
`rullst-mail` and `rullst-ai`; the peer-address, status and content-type checks
of `EgressFetcher::fetch_bytes` run only after a connection to a public HTTPS
origin, which the egress policy refuses to loopback test servers, and are left
to live integration testing; in `rullst-core`, the `pool_ready` guard of
`run_artisan_command` needs an already initialized ORM pool, which the
all-feature build can only open against a PostgreSQL server:
`rullst-core/tests/artisan_open_pool.rs` opens one in a PostgreSQL
testcontainer and runs `db:status` in a child process, so runs with Docker
measure the guard, and runs without it skip the test unless
`RULLST_REQUIRE_TESTCONTAINERS=true`), the `cargo-rullst` campaign (targeted
runs, with the CLI's library unit tests, of the self-update, audit, `add` and
consumer-file, doctor, deploy and `upgrade` files, and before 13.0.0-alpha.4
of the `green` command, `generators/deploy.rs` and `update/guided.rs`; it
covers the files `.github/mutants-pr-cli.txt` lists, and the campaign status above names the
survivors left in the others) and the Studio remote-mode campaign (targeted runs of every
`rullst-studio/src/remote` file and of the `rullst-studio` binary with the
package's tests) gave every missed mutant in their files a test, except these.
Each change has no observable effect in the all-feature Linux build the
campaign measures, so a test cannot tell it apart. Keep this list short: a new
entry needs the same one-line justification. The pull-request gate (`mutants-pr.yml`) skips an
entry only when its exact cargo-mutants name is also listed in
`.github/mutants-pr-equivalents.txt`.

| Mutant | Why it is equivalent |
| --- | --- |
| `rullst-core/src/queue/deferral/window.rs`: `TimeWindow::length_ms`, `>` with `>=` | `TimeWindow::daily` rejects equal start and end times, so the two never compare equal. |
| `rullst-core/src/queue/deferral/window.rs`: `TimeWindow::intervals`, `day < until_ms` with `<=` | The extra day starts at or after `until_ms`, so it never yields an interval. |
| `rullst-core/src/testing/html_normalize.rs`: `mask_csrf_headers`, `found < copied` with `<=` | `copied` indexes the closing quote of a masked value, so no `x-csrf-token` key can start there. |
| `rullst-media/src/s3/signing.rs`: `settings` with `Default::default()` | Adapter URLs hold only unreserved characters, `%XX` and `/` without empty or dot segments, so single or double encoding and path normalisation give the same canonical URI. |
| `rullst-mail/src/facade/driver.rs`: `DigestHasher::finish` with `1` | `finish` is never called; the digest is read from the SHA-256 state. |
| `rullst-mail/src/facade/driver.rs`: delete the `(Some(_), Some(_))` arm of `ses_credentials` | That arm exists only without the `aws-ses` feature, which the all-feature build does not compile. |
| `cargo-rullst/src/generators/audit_report/a11y.rs`: `attributes`, the end `index + 1` after `>` with `index - 1` or `index * 1` | The tag scanner resumes at the next `<`; the byte before a closing `>` cannot open a tag, because a tag name never starts with `>`. |
| `cargo-rullst/src/generators/green/procfs.rs`: `ticks_per_second`, `> 0` with `>= 0` | The clock tick rate is always positive. |
| `cargo-rullst/src/generators/green/procfs.rs`: the non-Unix `ticks_per_second` with `Some(0)` or `Some(1)` | Not compiled on Linux. |
| `cargo-rullst/src/generators/green/load.rs`: `worker`, `green/artifacts.rs`: `inspect`, and `green/target.rs`: `OwnedApp::wait_ready` and `OwnedApp::stop`, the deadline test `Instant::now() < deadline` with `<=` | An `Instant` reading would have to equal the deadline to the nanosecond, and the loop would then take one more short step. |
| `cargo-rullst/src/generators/doctor/probe.rs`: `run`, `stdout.len() > OUTPUT_LIMIT` with `>=` | An output of exactly `OUTPUT_LIMIT` bytes is cut at its own length, which leaves it unchanged. |
| `cargo-rullst/src/generators/doctor/database.rs`: `tcp_reachable`, delete the `TimedOut \| WouldBlock` arm | `TcpStream::connect_timeout` reports its timeout as a `TimedOut` error whose text is `connection timed out`, which the catch-all arm passes through unchanged, and never returns `WouldBlock` on Linux. |
| `cargo-rullst/src/generators/output_guard.rs`: `write_output`, the `NotFound` guard of `fs::metadata` with `true` | `reject_symlink` has already returned every other `lstat` error, and for a path that is not a symlink `stat` fails exactly when `lstat` does. |
| `cargo-rullst/src/generators/output_guard.rs`: `write_output`, the `!parent.as_os_str().is_empty()` guard with `true` | `tempfile` creates its file for the empty directory `""` in the current directory, as it does for `"."`. |
| `cargo-rullst/src/generators/consumer_files.rs`: `reject_links`, the `NotFound` guard with `true` | Any other `lstat` failure of an ancestor (permission, not a directory, name too long) recurs with the same error when the path itself is read or created. |
| `cargo-rullst/src/update/project/permissions_unix.rs`: `xattr_names`, `size > MAX_XATTR_NAMES` with `==` or `>=` | Linux never reports a name list longer than 64 KiB, and a list of exactly 64 KiB cannot hold only `security.selinux`, so `has_unreproduced_attribute` refuses it with the same error. |
| `cargo-rullst/src/update/cache/unix.rs`: `options`, `NOFOLLOW \| NONBLOCK` with `^` | The two flags occupy disjoint bits, so OR and XOR give the same value. |
| `cargo-rullst/src/update/cache/unix.rs`: `load_at`, `FILE_LIMIT + 1` with `-` or `*` | `validate_file` has already refused a file longer than `FILE_LIMIT`. Reading one byte less shortens only a file of exactly `FILE_LIMIT` bytes, whose body `decode` refuses either way because it allows 64 bytes less, and a file that grows during the read is refused either way. |
| `cargo-rullst/src/generators/audit_source.rs`: `tokenized_views` and `mark_code_tokens`, `index < tokens.len()` with `<=`, and `tokenized_views`, `index = last + 1` with `last - 1` or `last * 1` | At `tokens.len()` neither attribute lookup finds a token, so the loop steps past the end or stops. Resuming on an item's last token or the one before it revisits its terminating `;` or `{ .. }`, which cannot start `#[cfg(test)]`; at the end of the input a re-found attribute only adds a range inside the item already blanked. |
| `cargo-rullst/src/generators/build/upgrade/backup/restore.rs`: `is_link` with `false`, and its Windows reparse-point test with `&` replaced by `\|` or `^` or `!=` by `==` | On Linux `lstat` never reports a symlink as the regular file or directory that `validate_file` and `validate_directory_creation` also require, and the Windows test is not compiled. |
| `cargo-rullst/src/generators/build/upgrade/backup/restore.rs`: `restore_from`, the index read's `MAX_INDEX_BYTES + 1` and the staging copy's `expected_size + 1` with `*` | The index and each snapshot have already been bounded by their metadata, so reading one byte more matters only for a file that grows between that check and the read, which a test cannot arrange. |
| `cargo-rullst/src/generators/deploy_doctor/input.rs`: `linked` with `false` and its Windows reparse-point test; in `read`, the ancestor `linked(..) \|\| !is_dir()`, the file `!is_file() \|\| linked(..) \|\| len > MAX_FILE` and the opened-file re-check with `&&`, `NOFOLLOW \| NONBLOCK` with `&` or `^`, and `MAX_FILE + 1` with `*` | On Linux `lstat` never reports a symlink as a file or directory, and the Windows test is not compiled. A non-directory ancestor fails the final lookup with `NotADirectory`, and a directory, FIFO or oversized file that passes a weakened check is refused after opening or reading with the same `File` error. The open flags, the re-check and the extra byte matter only when the file is replaced or grows between `lstat`, `open` and the read, which a test cannot arrange. |
| `cargo-rullst/src/update/project/snapshot.rs`: the non-Windows `linked` with `false`; in `read`, `NOFOLLOW \| NONBLOCK` with `&` or `^`, the opened-file re-check `!opened.is_file() \|\| linked(..) \|\| len != ...` with `&&`, and `MAX_FILE + 1` with `*` | `lstat` never reports a symlink as a file or directory, so the type checks refuse links either way. `read` has already refused links, special files and files over 64 MiB, so the open flags, the re-check after opening and the extra byte matter only when the file is replaced or grows between `lstat`, `open` and the read, which a test cannot arrange. |
| `rullst-studio/src/remote/history.rs`: `Sample::from_outcome`, `window.requests > 0` with `>=` | With no request the rate is `0 / 0`, which `finite` drops, so the sample stores no rate either way. |
| `rullst-iot/src/coap.rs` (`CoapRequest::encode` twice, `append_option`), `rullst-iot/src/mqtt.rs` (`MqttPublish::encode`) and `rullst-iot/src/i2c.rs` (`I2cHelper::try_build_read_frame`): `\|` with `^` | The operands occupy disjoint bits, so OR and XOR build the same byte. |
| `rullst-iot/src/coap.rs`: `CoapRequest::encode`, `payload.len() > MAX_COAP_DATAGRAM_BYTES` with `==` or `>=` | `CoapRequest::payload` already bounds the payload, and `ensure_room` rejects the same lengths with the same error. |
| `rullst-iot/src/mqtt.rs`: `MqttPublish::encode`, `remaining_length > MQTT_MAX_VARIABLE_BYTE_INTEGER` with `==` or `>=` | The builders keep topic and payload far below that limit, and `variable_byte_integer` repeats the check with the same error. |
| `rullst-privacy/src/age_assurance/challenge_tokens.rs`: `ChallengeTokens::open_with_clock`, `token.len() > MAX_TOKEN_BYTES` and `payload.len() > MAX_PAYLOAD_BASE64` with `==` or `>=`, the `\|\|` before `payload.is_empty()`, the payload bound and `tag.len() != 43` with `&&`, and `MAX_PAYLOAD_BASE64` with its `+` or `/` replaced by `*` | These pre-authentication bounds only reject early what key lookup, tag verification, the 4096-byte decoded limit or JSON parsing reject with the same `InvalidChallengeToken`: a well-formed token is far below 8192 bytes, and 5463 base64 characters already decode to 4097 bytes. |
| `rullst-privacy/src/age_assurance/sqlite.rs`: `SqliteReplayStore::open`, the `error.kind() != NotFound` guard with `false` | Every other metadata failure (permission, name length, symlink loop) also makes the SQLite open fail with the same `StoreUnavailable`. |
| `rullst-privacy/src/consent/gate.rs`: `allows_with_clock`, `completed < record.valid_until()` with `<=` | A grant with `completed >= valid_until` has already returned `false`, so the two never compare equal there. |
| `rullst-labs/src/sqlite/completion.rs`: `finish`, `lease.until <= tx.now \|\| record.view.expires_at <= tx.now` with `&&` | `narrow_deadline` with the earlier of the two returns the same `Expired`. |
| `rullst-labs/src/sqlite/jobs.rs`: `cancel`, `revision <= 0 \|\| record.view.revision != revision` with `&&` | `save_job` updates only the row at that revision, so a stale revision still returns `Conflict`. |
| `rullst-labs/src/sqlite/jobs.rs`: `load_job`, `body.len() > 32796`, and `seal_record`, `body.len() > 32768`, each with `==` or `>=` | Bounded record fields serialize far below 32 KiB, and a longer stored body fails authentication with the same `Integrity`. |
| `rullst-labs/src/sqlite/jobs.rs`: `payload`, the `\|\|` before the scope and id comparisons with `&&` | The exercise digest covers scope, id and revision, so none of them can differ alone. |
| `rullst-labs/src/sqlite/retention.rs`: `expire_queued`, either `\|\|` with `&&`, and `purge_terminal`, the `\|\|` after the state match with `&&` | The SQL selection, `load_job`'s column checks and `Record::validate` already guarantee a queued, expired, unleased record, or an unleased one that is neither queued, running nor `Uncertain` (which always holds its lease). |
| `rullst-supervision/src/exam/observation.rs`: `Observation::from_code`, `(code - 10) % 4` with `+` | `(code + 10) % 4` equals `(code - 10) % 4`, because 20 is a multiple of 4. |
| `rullst-supervision/src/sqlite/events.rs`: `events`, and `rullst-supervision/src/sqlite/observations.rs`: `observations`, `expires_at > MAX_TIME` with `==` or `>=` | An event is received before its session expires, at least an hour before the session's retention ends at or before `MAX_TIME`, so with its fixed retention it always expires before `MAX_TIME`. |
| `rullst-supervision/src/sqlite/observations.rs`: `record_observation`, both source match guards with `true` | `record_browser` and `record_capture` pass only browser or capture observations with the browser source, and `analyze` only presence or audio findings with an adapter source. |
| `rullst-supervision/src/sqlite/transaction.rs`: `Operation::begin`, `now < before \|\| now < previous_time` with `&&` | `finish` checks the time against the same high-water mark and returns the same `Clock`. |
| `rullst-supervision/src/sqlite/mod.rs`: `resolved_path`, both metadata match guards with `false` | `open` checks for a regular file again, `initialize` creates the file exclusively and fails on any existing entry, and every other metadata error also fails the open with the same `Storage`. |
| `rullst-auth/src/auth/app_key.rs`: `parse_dotenv`, delete the `dotenvy::Error::Io` arm | The parser reads an in-memory `&str`, and valid UTF-8 read line by line cannot raise an I/O error. |
| `rullst-auth/src/auth/passkey/ceremony.rs`: `ChallengeStore::issue`, `challenge.expires_at > now` with `>=` | It keeps only a challenge whose expiry equals a later `Instant::now()` reading to the nanosecond, which `consume` already rejects as expired. |
| `rullst-auth/src/auth/passkey/shared/manager.rs`: `SharedPasskeyAuth::start_authenticate`, `allowed.is_empty() \|\| allowed.len() > MAX_CREDENTIALS` with `&&` | `CeremonyIntent::new` rejects an empty or 33-entry authentication allowlist with the same `InvalidInput` before the store is called. |
| `rullst-auth/src/auth/passkey/shared/manager.rs`: `validate_passkey`, the `\|\|` before `first() != Some(&4)`, and `rullst-auth/src/auth/passkey/sqlite.rs`: `validate_passkey`, `\|\|`, each with `&&` | A key that `from_sec1_bytes` accepts has either 65 bytes and tag 4 or neither, so a key failing only one check fails parsing with the same error. |
| `rullst-auth/src/auth/passkey/verification.rs`: `verify_assertion`, `FLAG_ATTESTED_CREDENTIAL_DATA \| FLAG_EXTENSION_DATA` with `^` | The flags occupy disjoint bits, so OR and XOR build the same mask. |
| `rullst-auth/src/recovery/connection.rs`: the fallback `configure_postgres` and `configure_sqlite` with `Ok(())` | They compile only without `recovery-postgres` or without both SQLite families, which the all-feature build does not compile. |
| `rullst-auth/src/recovery/email_login/delivery.rs`: `<impl Drop for WireNotice>::drop` with `()` | Zeroizing a buffer that is being freed has no effect safe code can observe. |
| `rullst-auth/src/recovery/email_login/types.rs`: `safe_path`, `part == "." \|\| part == ".."` with `&&` | The byte allowlist that follows already rejects every `.`. |
| `rullst-auth/src/recovery/store.rs`: `SqlRecoveryStore::connect`, the `\|\|` before the `recovery-postgres` clause with `&&` | With every feature enabled both feature clauses are always false, so joining them with `&&` changes nothing. |
| `rullst-security/src/telemetry.rs`: `SecurityStore::push_live_event`, `events.len() > MAX_LIVE_EVENTS` with `>=` | Truncating a buffer that holds exactly 50 events to 50 changes nothing. |
| `rullst-security/src/telemetry/event.rs`: `security_event_schema_version` with `1` | `SECURITY_EVENT_SCHEMA_VERSION` is 1. |
| `rullst-security/src/telemetry/event.rs`: `truncate_utf8`, `boundary > 0` with `>=` | `is_char_boundary(0)` is always true, so the loop stops at 0 either way. |
| `rullst-security/src/sentinel/pow/codec.rs`: `decode_token`, `token.is_empty() \|\| token.len() > 128` with `&&`, and `>` with `==` or `>=` | Decoding rejects every token that does not decode to exactly 82 bytes with the same `InvalidToken`. |
| `rullst-security/src/siem/spool.rs`: `decode_file`, and `rullst-security/src/siem/authenticated/codec.rs`: `read_and_verify`, `events.len() + 1` in the `invalid frame offset` error of `cursor.checked_add(relative_end)` with `-` or `*` | A line end found inside the buffer cannot overflow `usize`, so the closure never runs. |
| `rullst-security/src/recovery_codes.rs`: `GeneratedRecoveryCodes::drop` with `()` | Zeroizing the plaintext codes changes only memory that is freed right after; safe code cannot read it. |
| `rullst-security/src/mfa.rs`: `verify_totp_step_at`, `code.len() != 6 \|\| !all digits` with `&&` | Every other input fails the constant-time comparison with the six-digit expected code. |
| `rullst-security/src/mfa.rs`: `decode_base32` and `generate_totp_at_counter` (three times), `\|` with `^` | The operands occupy disjoint bits, so OR and XOR build the same value. |
| `rullst-security/src/schema_guard.rs`: `StrictJsonVisitor::expecting` with `Ok(Default::default())` | serde_json calls a visit method for every JSON value, so the message is never formatted, and `inspect_json_payload` maps every error to fixed text. |
| `rullst-security/src/dlp/masking.rs`: `mask_database_passwords`, `pass_end + 1` with `-` or `*` | The next scheme search starts at or just before the `@`, where no scheme can begin. |
| `rullst-security/src/log_redactor.rs`: `identifier_continuation`, `end + 1` with `-` | A `_` or `.` followed by a non-alphanumeric byte then yields an empty last component, which is skipped like the identifier without it: neither byte is a separator, quote or space. |
| `rullst-security/src/timing_guard.rs`: `TimingScope::finish`, `elapsed < target_duration` with `<=` | Sleeping for a zero remainder returns immediately. |
| `rullst-security/src/login_guard.rs`: `record_login_failure`, the window match guard with `true` | `prune_expired` has just removed every counter whose last attempt is a window or more old. |
| `rullst-security/src/rate_limit.rs`: `is_rate_limited_in`, the retention `<` with `<=` | Two `Instant::now()` readings would have to differ by exactly the window, and an entry kept at that instant restarts its count on its next use. |
| `rullst-core/src/security/pii.rs`: `decode_json_char`, delete the `"`, `\`, `/`, `b`, `f`, `n`, `r` or `t` arm | Each of these escapes decodes to a character that, like the opaque stand-in, is neither a digit, a card separator, an address character nor `@`, and both keep the escape's original spelling. |
| `rullst-core/src/security/pii.rs`: `mask_chars`, `idx - start` with `+` and `username_len > 1` with `>=` | A local part of at most one character has nothing to mask after its first character, and skipping to the end of the domain passes only domain characters, never another `@`. |
| `rullst-core/src/security/pii.rs`: `mask_chars`, the domain slice start `idx + 1` with `idx * 1` or `idx - 1` | The domain holds a dot, so the leading `@` never reaches the top-level domain; when its only dots are trailing ones, the text after the last dot starts with `@` and fails both domain tests. |
| `rullst-core/src/security/csrf_multipart.rs`: `scan`, `available < first.len()` with `>` | A prefix shorter than the first delimiter reaches the loop's two-byte read past its end, which returns the same `Incomplete`. |
| `rullst-core/src/security/csrf.rs`: `is_csrf_exempt_path`, the first two `\|\|` with `&&` | `/robots.txt`, `/sitemap.xml` and `/favicon.ico` stay exempt through their `.txt`, `.xml` and `.ico` suffixes. |
| `rullst-core/src/queue/mod.rs`: the default `QueueDriver::push_at`, `available_at > SystemTime::now()`, and `Queue::dispatch_at`, `delay > MAX_SCHEDULE_DELAY`, each with `>=` | They differ only when the requested time equals a clock reading taken inside the call, which a caller cannot arrange. |
| `rullst-core/src/queue/worker/execution.rs`: `record_outcome`, the `error.is_panic()` guard with `true` | `execute_job` handles every cancelled handler before it calls `record_outcome`, so the only join error left is a panic. |
| `rullst-core/src/storage/tenant.rs`: `confine_local_root`, the `!create && NotFound` guard with `false` or without `!` | Below a missing tenant directory, `get`, `metadata` and `delete` then fail with the same `NotFound` that the operation itself reports; `put` creates the directory first. |
| `rullst-core/src/cache/memory.rs`: `MemoStore::get`, `now < entry.expires_at`, and `MemoStore::set`, the expiry `retain`'s `<`, each with `<=` | An `Instant` reading would have to equal the entry's expiry to the nanosecond. |
| `rullst-core/src/lifecycle.rs`: `begin_draining`, `in_flight_requests() == 0` with `!=`, and `release_request`, `&&` with `\|\|` | `wait_for_drain` waits only on a draining or stopped lifecycle and re-reads the count after every wake-up, so a wake-up that no waiter can receive, or an extra one, changes nothing. |
| `rullst-core/src/storage/cloud/multipart/codec.rs`: `MultipartStorage::open`, `checkpoint.0.len() > MAX_TOKEN` with `==` or `>=`, and `bytes.len() < 28` with `<=` | No authentic checkpoint reaches 256 KiB (256 receipts and a 2,048-byte upload id stay under 180 KiB), and a 28-byte token has an empty plaintext; both fail with the same `InvalidCheckpoint`. |
| `rullst-core/src/testing/snapshot.rs`: `line_diff`, the deletion test's `table[i * width + j + 1]` with `j * 1` | Without a match, `table[i][j]` is the larger of `table[i + 1][j]` and `table[i][j + 1]`, so `table[i + 1][j] >= table[i][j]` holds exactly when `table[i + 1][j] >= table[i][j + 1]`. |
| `rullst-core/src/queue/deferral/planner.rs`: `lowest_intensity`, the tie-break `overlap_start < best_start` with `<=` | A tie at the same start stores the same start and value again. |
| `rullst-core/src/live/recovery/types.rs`: the duplicate-free field visitor's `expecting` with `Ok(Default::default())` | `LiveCommand::decode` turns every deserialization error into the same `Invalid`, so the expectation text is never seen. |
| `rullst-core/src/rpc.rs`: `request_contract_failure`, delete the `BodyTooLarge` arm | `to_bytes` already refuses a body over the same limit with the same 413 and code, so decoding never reports it. |
| `rullst-core/src/rpc.rs`: `RpcFailure::from_detail` with `Default::default()` | Compiled only for `wasm32`. |
| `rullst-core/src/rpc.rs`: the test helper `failure_for`, `64 * 1024` with `+` | Test code: every failure envelope it reads is far below 1,088 bytes. |
| `rullst-core/src/storage/cloud/multipart/protocol.rs`: `parse`, delete the `allow_dtd` field | roxmltree's default `ParsingOptions` already rejects a DTD. |
| `rullst-core/src/live/recovery/session.rs`: `run`, delete the `Ok(Ok(()))` arm | A session ends normally only when the peer closed: tungstenite answers a close frame itself, so the code chosen here never reaches the peer. |
| `rullst-core/src/live/recovery/session.rs`: `send_snapshot`, `encoded.len() > 512 * 1024` with `==` or `>=` | A snapshot holds at most 64 KiB of HTML, which JSON escaping expands to under 400 KiB. |
| `rullst-core/src/cache/memory/exact_json.rs`: `Display for Inexact` with `Ok(Default::default())` | `is_exact_json` only checks whether serialization succeeds; the error text is never formatted. |
| `rullst-core/src/radar.rs`: `get_windows_memory_mb`, `get_windows_process_cpu_usage`, `filetime_100ns`, `calculate_windows_cpu_percent`, and the Windows and other-platform `get_process_cpu_usage` | Not compiled on Linux. |
| `rullst-core/src/feature/db.rs`: `DbFeatureDriver::resolve_with`, the freshness test `Instant::now() < entry.expires_at`, and `remember`, the eviction `retain`'s `<`, each with `<=` | An `Instant` reading would have to equal the entry's expiry to the nanosecond. |
| `rullst-core/src/cache.rs`: `MemoryDriver::cleanup_if_due`, the sweep's `now < expires_at`, and `inspect`, the filter's `expires_at > now`, with `<=` and `>=` | An `Instant` reading would have to equal the entry's expiry to the nanosecond. |
| `rullst-core/src/storage/local_paths.rs`: `reject_symlink_components`, the `NotFound \| NotADirectory` guard with `true` | Any other `symlink_metadata` failure (permission, name length) recurs in the `canonicalize`, `create_dir_all` or write that follows, with the same `Io` text. |
| `rullst-core/src/telemetry/distributed/processor.rs`: `MinimizedProcessor::on_start` with `()` | The wrapped `BatchSpanProcessor::on_start` ignores its arguments (opentelemetry_sdk 0.33). |
| `rullst-core/src/server/dylib_loader.rs`: `is_expected_loaded_library_removal_error` with `true` or `false` | Not compiled on Linux. |
| `rullst-core/src/config/project_setting.rs`: `parse_dotenv_entries`, delete the `dotenvy::Error::Io` arm | The entries are read from a `&str`, which never fails with an I/O error. |
| `rullst-nexus/src/nexus/crud/form.rs`: `form_html`, the `Password` guard with `false` | `render_field_widget` renders a `Password` input empty and without a placeholder, whatever value or state it receives. |
| `rullst-capital/src/invoice.rs`: `money_to_minor`, `(scaled - rounded).abs() > 1e-7` with `>=` | When `scaled` rounds to at least one minor unit, `scaled - rounded` is a multiple of 2^-53 and never equals the `f64` `1e-7`, so the two differ only for an amount that rounds to zero minor units, which `rounded < 1.0` rejects either way. |
| `rullst-capital/src/invoice/pdf.rs`: `render_pdf`, `bytes.len() > MAX_PDF_BYTES` with `==` or `>=`, and the `\|\|` with `&&` | The embedded font is at most 8 MiB and a bounded invoice adds well under 1 MiB, so the document never reaches 16 MiB, and printpdf always writes the `%PDF-` header. |
| `rullst-capital/src/providers/mod.rs`: `http_client` with a leaked default client | Adapters only build requests with it; `execute` always sends them through the bounded client from `http::client`. |
| `rullst-capital/src/providers/stripe.rs`: `create_customer_portal`, `\|\|` with `&&` | `require_mock_operation` accepts the same empty and `mock_*` keys and the same fixture URL is returned; every other key fails there with the same error. |
| `rullst-orm/src/value.rs`: the SQLx `Any` pool's `temporal::date_time`, `naive_date_time`, `date` and `time` with `String::new()` or `"xyzzy".into()` | They exist only without a strict driver feature, and the all-feature build selects the PostgreSQL pool; `the_any_pool_binds_dates_as_iso_text` checks their exact text in default-feature builds. |
| `rullst-orm/src/schema/schema_builder.rs`: `prepare_postgres_enums`, delete `!` in the `!definitions.is_empty()` guard | That guard exists only without `strict-postgres`, which the all-feature build enables; `postgres_enum_preparation_validates_before_any_statement` covers it in default-feature builds. |
| `rullst-orm/src/pool/dsn.rs`: `percent_decode_utf8`, `(high << 4) \| low` with `^` | The high and low hex digits occupy disjoint bits, so OR and XOR build the same byte. |
| `rullst-orm/src/privacy.rs`: the default `ComplianceModel::personal_fields` with `vec![]` | The default already returns an empty vector. |
| `rullst-orm/src/polyglot/turso/model.rs`: the default `TursoModel::opaque_columns` with `Vec::leak(Vec::new())` | The default already returns an empty slice. |
| `rullst-orm-macros/src/feature_gates.rs`: `redis` and `ai` with `Default::default()` | With every feature the runtime gates are on and the feature is enabled, so both return an empty token stream, which is the default. |
| `rullst-orm/src/polyglot/recovery/codec.rs`: `base64url_value`, the digit arm's `byte - b'0'` with `+` | The value is read only for its low four padding bits, and the two results differ by 96, a multiple of 16. |
| `rullst-orm/src/polyglot/qdrant/mod.rs`: `LiveQdrant::send_json`, `bytes.len() > MAX_REQUEST_BYTES` with `==` or `>=` | No request reaches 4 MiB: vectors hold at most 65,536 values and payloads at most 1 MiB, so the body stays near 2 MiB; a test still sends a 1 MiB payload. |
| `rullst-orm/src/outbox.rs`: `Outbox::install`, `case_insensitive_keys > 0` with `>=` | The MySQL key-collation conversion is idempotent: run on keys that are already `ascii_bin` it leaves the same columns and only costs a table rebuild. The MySQL matrix converts a legacy table, which kills the `==` and `<` mutants. |
| `rullst-orm/src/schema/migration_lock.rs`: `MigrationLock::release` with `()` | The lock lives in the dedicated connection's session. The replaced body still drops `self`, which closes the socket and ends the session, freeing the lock as the graceful close does. |
| `rullst-messaging/src/webhooks/signature.rs`: `WebhookSignature::from_headers`, the `\|\|` before `kind.len() > 128`, `timestamp.len() > 12` or `signature.len() != 46`, and `tag.len() != 32 \|\| …`, each with `&&` | `MessageId::from_stored`, `EventKind`, the canonical timestamp parse and the strict `v1=` decode repeat each length check with the same `InvalidSignature`, and 43 strict base64url characters always decode to 32 canonical bytes. |
| `rullst-messaging/src/sqlite/storage/crypto.rs`: `open`, the `minimum` sum with a `+` replaced so that it shrinks, and `envelope.len() < minimum` with `<=` or `==` | The shortest authentic envelope is 38 bytes; a shorter one fails the key-ID, nonce or tag parse with the same `StorageAuthenticationFailed`. |
| `rullst-messaging/src/schedules/crypto.rs`: `open`, `value.len() > MAX_CONTENT_BYTES + 128` with `==` or `>=` or its `+` with `*`, the `\|\|` after it with `&&`, and a `+` in `8 + 1 + 1 + 12 + 16` replaced so that the sum shrinks | Sealed values span 38 to 131,173 bytes, so each change only moves which later parse or authentication step rejects a value, with the same `Encryption`. |
| `rullst-messaging/src/schedules/delivery.rs`: `verify`, `lease.namespace != … \|\| now >= lease.expires` with `&&` | Occurrence IDs digest their namespace, so another namespace's lease is not found by its ID, and an expired lease fails the commit deadline; both return the same `InvalidLease`. |
| `rullst-messaging/src/wire.rs`: `FIXED_BYTES`, a `2 + 2` with `2 * 2`, and `Reader::is_finished` with `true` | `2 * 2` equals `2 + 2`; `decode` re-encodes the envelope and compares it with the frame, which rejects trailing bytes with the same `InvalidWireEnvelope`. |
| `rullst-messaging/src/redis_streams/transport.rs`: `Remote::decode`, delete the `"missing" \| "corrupt"` arm | The catch-all arm returns the same `CorruptStorage`. |
| `rullst-connect/src/provider/id_token.rs`: `validate_claims_with_presenters`, the `\|\|` before `nonce.is_empty()` with `&&` | `verify_nonce` compares SHA-256 digests, so a nonce empty on one side only already fails it, and two empty nonces still satisfy the joined condition. |
| `rullst-mail/src/security.rs`: `redact_labelled_private_keys`, `MAX_PEM_LABEL_BYTES + 1` with `*` | Reading at most 64 label bytes cuts a longer label inside its run of label bytes, so the `-----` check that follows fails as the length check would. |
| `rullst-mail/src/security.rs`: `href_values`, `value_start + length + 1` with `-` or `*` | The scan resumes on the closing quote or the byte before it (the opening quote for an empty value), and no `href` match can contain a quote. |
| `rullst-mail/src/inspection/sniff.rs`: `root_element`, `open < close` with `<=` | Both are positions just after one byte, a `[` and a `>`, so they are never equal. |
| `rullst-ai/src/ai/egress.rs`: `EgressPolicy::strict` with `Default::default()` | `strict` already returns `Self::default()`. |
| `rullst-ai/src/ai/egress.rs`: `is_public_ipv6`, `is_unspecified() \|\| is_loopback()` with `&&` | `::` and `::1` lie outside `2000::/3`, which the next clause rejects. |
| `rullst-ai/src/ai/guardrails/markdown_inline.rs`: `open_tag_end`, the self-closing `then_some(index + 1)` with `-` or `*` | The tag then ends on or just before its `/`; the byte before that `/` is whitespace, a quote or a name byte, never a bracket. |
| `rullst-ai/src/ai/guardrails/markdown_images.rs`: `definition_destination`, the `<...>` end `end + 1` with `*` | It drops only the closing `>` or line ending, which neither the scheme nor the authority check reads. |
| `rullst-ai/src/ai/guardrails/markdown_code.rs`: `indentation`, the tab step `4 - column % 4` with `4 + column % 4` | Indentation is only compared with 4, and either step takes a tab to at least four columns. |
| `rullst-ai/src/ai/guardrails/tax_ids.rs`: `token_end`, `bytes.get(index + 1)` with `index - 1` | A separator then joins when a digit precedes it instead of follows it, but the end moves only on digits and a second separator stops both scans, so every token ends at the same digit. |
| `rullst-ai/src/ai/guardrails/tax_ids.rs`: `is_valid_tax_id`, the CNPJ `all_digits && token.len() == 14` with `\|\|` | `digit_values` still requires exactly 14 digits, which only a 14-digit token or the CNPJ layout holds. |

Timed-out mutants are not counted as caught, and cargo-mutants reports them
separately. The campaign rewrote the scanning loops of the WAF detector, the
RASP decoder, the HTML snapshot normaliser and the accessibility scanner so
that every step consumes input, and bounded the WAF linear-time test, which
removed their timeouts. The core campaign ran the queue worker, realtime,
multipart CSRF, byteranges and long PII masking tests against a deadline
instead, which turned their timeouts into caught mutants; the CLI campaign
bounded the update cache's FIFO test the same way. The remaining ones
turn forward progress into a loop that never ends without allocating, in code
every test of the file reaches, so no single test can fail first:

- `rullst-core/src/error_console/store.rs`: `Store::record` with
  `len() >= MAX_ENTRIES` replaced by `<` pops an empty queue forever.
- `cargo-rullst/src/generators/build/upgrade/rules/tokens.rs`: the token
  scanner's `level` and `path` loops, where an index step (`+= 1`, `+= 2`,
  `next + 1`, `end + 1`, `end + 2`) becomes `-` or `*`, or `path` returns
  `0` or `1`, and the scan revisits the same token forever.
- `cargo-rullst/src/generators/audit_source.rs`: the token scans of
  `tokenized_views`, `item_end`, `mark_code_tokens` and `doc_attribute`, where
  an index step (`+= 1`) becomes `-=` or `*=`, `item_end` returns `0` or `1`,
  or `doc_attribute` returns `Some(0)` or `Some(1)` or its `next + 1` becomes
  `-`, and the scan revisits the same token forever.
- `rullst-security/src/log_redactor.rs`: the bearer, identifier, separator
  and escaped-quote scans, where an index step (`+= 1`, `+= 2`) becomes `-`
  or `*`, a key offset (`cursor + offset`, `key_start +
  key.len()`) becomes `-` or `*`, or `identifier_continuation` returns `0` or
  `1`, and the scan finds the same key or byte again forever.
- `rullst-security/src/dlp/byteranges.rs`: `Lines::next`, where the
  non-empty filter is negated or the next-line offset (`start + offset + 1`)
  becomes `-` or `*`, and the reader returns the same line forever;
  `rullst-security/src/dlp/masking.rs`: the AWS-key and database scans, where
  `cursor + offset` becomes `*` or `-`, or `+ scheme.len()` becomes `-`, and
  the scan restarts before the same match.
- `rullst-core/src/security/pii.rs`: `measure_digit_run` returning `(0, 0)`,
  `(0, 1)`, `(1, 0)` or `(1, 1)`, its `separators < 3` with `==` or `>`, or
  its `end += 1` with `*=` or `-=`, and the address and JSON-string scans of
  `mask_chars` and `mask_json_strings`, where `start -= 1` becomes `/=`,
  `end += 1` or `idx += 1` becomes `*=`, or the dotted-domain `end += 1`
  becomes `-=`, so the card, address or string scan never advances; and
  `rullst-core/src/security/pii_byteranges.rs`: `Lines::next`, where the
  next-line offset becomes `-` or `*`, as in the DLP reader above.
- `rullst-core/src/server/dev_telemetry/recorder.rs`: `push_bounded` with
  `entries.len() >= capacity` replaced by `<` pops an empty queue forever, in
  code every recorded request or query reaches.
- `rullst-core/src/scheduler.rs`: `shutdown_requested` returning `true` or
  `false`, and `wait_for_shutdown` returning at once or without its `!`, so a
  task loop never runs or never sees shutdown, and the scheduler, server and
  supervision tests that start and stop a scheduler wait forever.
- `rullst-security/src/login_guard/state.rs`: `pop_earliest` returning
  `None` or `Some(Default::default())`, or `make_room` with `&&` replaced by
  `||`, so pruning or eviction never removes an entry; and
  `rullst-security/src/rasp/user_agent.rs`: the version scan's `index += 1`
  replaced by `*=`.
- `rullst-mail/src/security.rs`: the redaction scans, where
  `start + relative_end` in `redact_values_after` becomes `-`, `offset +
  relative` in `redact_aws_access_keys` or `redact_labelled_private_keys`
  becomes `*`, or `start + prefix.len()` becomes `-` or `*`, and the
  `href_values` scan, where `index += 1` in `skip_whitespace` or the
  non-match `pos += 1` becomes `*=`, or the `pos += 4` after an `href` without
  `=` becomes `-=`; each scan restarts before the same match.
- `rullst-ai/src/ai/guardrails/tax_ids.rs`: `mask_tax_ids` without the `!`
  of its digit test or with `start += 1` as `-=` or `*=`, `token_end`
  returning `0` or `1`, and its `index += 1` steps as `-=` or `*=`;
  `rullst-ai/src/ai/guardrails/markdown_inline.rs`: `code_span_hides_bracket`
  with either backtick test negated, `index += run` as `-=` or `*=` or
  `index += 1` as `*=`, and the whitespace steps of `open_tag_end` as `*=`;
  `rullst-ai/src/ai/guardrails/markdown_images.rs`: the `index += 1` steps of
  `unescaped_close` and `label_end` as `-=` or `*=`; and
  `rullst-ai/src/ai/guardrails/markdown_code.rs`: the index steps of `lines`,
  `mark_fenced` and `span_contents` as `*=`, and `blank_code` with
  `code[group_start] || is_blank(...)` as `&&`, which stops on a blank line.
  The guardrail tests reach these scans, which then never finish.

`rullst-security/src/timing_guard.rs`: `TimingScope::finish` with the jitter
`%` replaced by `+` does not loop: it sleeps for up to 72 minutes in the
real-time padding tests.

A targeted run sets each test binary's address space to 8 GiB, so a loop that
does allocate aborts its tests and counts as caught instead.
`rullst-ai/src/ai/guardrails/markdown_code.rs`: `blank_code` with
`group_start += 1` replaced by `*=` does allocate, but it repeats one line
forever; on a blank line the output grows one byte per turn and reaches that
limit only after the timeout.
