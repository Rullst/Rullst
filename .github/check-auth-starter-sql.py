#!/usr/bin/env python3
"""Run `cargo rullst auth` on fresh Blank and API starters against SQL servers (Linux CI).

For PostgreSQL, MySQL and MariaDB, each in a disposable container bound to
loopback, with the CLI built from this checkout (`--cli`), for each starter:

1. `cargo rullst new --blueprint <blank|api> --database <server>` (its
   manifest depends on this checkout); `.env` gets the server's DATABASE_URL
   and `cargo rullst db:migrate` creates the starter's `users` table. Two rows
   are inserted as that table allows: a name, with the default timestamps.
2. `cargo rullst auth`, the sign-up routes mounted in `router()` as Tutorial 11
   (Step 2) and the SaaS starter do, and `cargo rullst db:migrate`. The API
   starter has no sign-up of its own: `cargo rullst auth` generates the same
   form sign-up for every starter (there is no `auth --api`), and the API
   starter's `Server` applies CSRF to it as to every route that is not a
   bearer machine endpoint.
3. SQL against the server: the six added columns with their types,
   nullability and defaults, the unique index `users_email_unique` (which
   rejects a duplicate), the `user-<id>@no-email.invalid` placeholders without
   a password, the starter's columns, indexes and rows unchanged, and the
   batch's migration records and its `password_reset_tokens` and
   `rullst_auth_sessions` tables.
4. The built application: a sign-up over HTTP opens /dashboard, stores an
   Argon2id hash and registers the login in `rullst_auth_sessions` (only a
   digest of the cookie's session secret is stored).
5. `cargo rullst db:rollback` restores the starter's columns, indexes, tables
   and migration records exactly (its sessions migration drops
   `rullst_auth_sessions`) and keeps every row; `db:migrate` then applies the
   batch again and gives every row, the signed-up one too, a placeholder.
6. The re-created `rullst_auth_sessions.user_id` references `users`: a
   session of an account that does not exist is refused, and deleting the
   signed-up account with SQL deletes its session (`ON DELETE CASCADE`).

The generated builds share `--target-dir` (the workspace target, which the
relational CLI profiles have just filled) and run offline from the
workspace's Cargo.lock, as .github/run-workspace-test-shard.sh prepares. The
images, clients and catalog queries are in auth_starter_servers.py.

Usage: check-auth-starter-sql.py --cli PATH [--server NAME ...] [--starter NAME ...] [--target-dir DIR]
"""

from __future__ import annotations

import argparse
import contextlib
import importlib.util
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.parse

from auth_starter_servers import SERVERS, Database, Failure, Row, Server, log, run

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
SPEC = importlib.util.spec_from_file_location("dast_generated_app", HERE / "dast-generated-app.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import dast-generated-app.py")
# Its loopback-only HTTP client keeps cookies by hand.
HTTP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HTTP)

APP_PORT = 3000
BUILD_TIMEOUT = 45 * 60
STARTER_MIGRATION = "m20260601000000_create_users_table"
STARTER_COLUMNS = ["id", "name", "created_at", "updated_at"]
STARTER_NAMES = ("Ada Lovelace", "Grace Hopper")
ACCOUNT_NAME = "Signup Smoke"
ACCOUNT_EMAIL = "signup-smoke@example.com"
ACCOUNT_COLUMNS_SUFFIX = "_add_account_columns_to_users_table.rs"
RESET_TOKENS_SUFFIX = "_create_password_reset_tokens_table.rs"
SESSIONS_SUFFIX = "_create_rullst_auth_sessions_table.rs"
# Tutorial 11's sign-up routes, mounted before the health probes (`Server`
# applies CSRF to every route).
SIGNUP_ROUTES = """        get("/register" => controllers::auth_controller::register_view),
    ]
    // Mounted by .github/check-auth-starter-sql.py as Tutorial 11 (Step 2)
    // describes, before the health probes.
    .route("/register", rullst::routing::post(controllers::auth_controller::register_submit)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))
    .route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    // `Server` adds CSRF, the secure headers and the WAF in every environment.
"""
# The last route of each starter's router and the lines after it.
ROUTER_TAIL = """    ]
    // `Server` adds CSRF, the secure headers and the WAF in every environment.
"""
LAST_ROUTES = {
    "blank": """        post("/clicked" => clicked),
""",
    "api": """        post("/api/messages" => create_message),
""",
}
STARTERS = tuple(LAST_ROUTES)
ROUTER_ANCHOR = LAST_ROUTES["blank"] + ROUTER_TAIL
ROUTER_WITH_SIGNUP = LAST_ROUTES["blank"] + SIGNUP_ROUTES
TIMINGS: list[tuple[str, float]] = []


def expect(condition: bool, message: str, detail: object = None) -> None:
    if not condition:
        raise Failure(message if detail is None else f"{message}\n{detail}")
    log(f"ok: {message}")


@contextlib.contextmanager
def step(title: str):
    """Collapses a step's output in the Actions log and records its duration."""
    log(f"::group::{title}")
    started = time.monotonic()
    try:
        yield
    finally:
        TIMINGS.append((title, time.monotonic() - started))
        log("::endgroup::")


def point_env_at(env_text: str, url: str) -> str:
    """The generated `.env` with its one DATABASE_URL line replaced."""
    lines = env_text.splitlines(keepends=True)
    matches = [index for index, line in enumerate(lines) if line.startswith("DATABASE_URL=")]
    if len(matches) != 1:
        raise Failure(f"the generated .env has {len(matches)} DATABASE_URL lines, expected 1")
    lines[matches[0]] = f"DATABASE_URL={url}\n"
    return "".join(lines)


def mount_signup_routes(main_rs: str, starter: str = "blank") -> str:
    """A starter's `src/main.rs` with the sign-up routes mounted."""
    anchor = LAST_ROUTES[starter] + ROUTER_TAIL
    if main_rs.count(anchor) != 1:
        raise Failure(f"src/main.rs no longer has the {starter} starter's router; update LAST_ROUTES")
    return main_rs.replace(anchor, LAST_ROUTES[starter] + SIGNUP_ROUTES, 1)


def placeholder_accounts(rows: list[Row]) -> list[Row]:
    """What `accounts` reads for rows that existed before the account columns."""
    return [(row[0], row[1], f"user-{row[0]}@no-email.invalid", "none", "none", "0", "0") for row in rows]


ACCOUNTS = ("SELECT id, name, email, CASE WHEN password_hash IS NULL THEN 'none' ELSE 'set' END, "
            "CASE WHEN oauth_provider IS NULL AND oauth_id IS NULL THEN 'none' ELSE 'set' END, "
            "CASE WHEN is_admin THEN 1 ELSE 0 END, session_epoch FROM users ORDER BY id")


class Application:
    """The generated project's debug binary, serving on the loopback port."""

    def __init__(self, binary: Path, project: Path, env: dict[str, str], log_path: Path):
        self.log_path = log_path
        self.output = log_path.open("w")
        environment = dict(env, HOST="127.0.0.1", PORT=str(APP_PORT))
        self.process = subprocess.Popen([str(binary)], cwd=project, env=environment, stdout=self.output,
                                        stderr=subprocess.STDOUT, start_new_session=True)
        self.base = urllib.parse.urlsplit(f"http://127.0.0.1:{APP_PORT}")

    def text(self) -> str:
        if not self.output.closed:
            self.output.flush()
        return self.log_path.read_text(errors="replace")

    def wait_until_ready(self) -> None:
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise Failure(f"the application exited with {self.process.returncode}:\n{self.text()[-4000:]}")
            with contextlib.suppress(HTTP.Failure):
                if HTTP.request(self.base, "GET", "/health", {})[0] == 200:
                    return
            time.sleep(1)
        raise Failure(f"/health did not answer 200 within 120 s:\n{self.text()[-4000:]}")

    def request(self, method: str, path: str, cookies: dict[str, str],
                form: dict[str, str] | None = None) -> tuple[int, dict[str, str], dict[str, str], str]:
        try:
            return HTTP.request(self.base, method, path, cookies, form)
        except HTTP.Failure as error:
            raise Failure(str(error)) from error

    def stop(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=15)
        self.output.close()


def migration_name(project: Path, suffix: str) -> str:
    matches = sorted((project / "src/migrations").glob(f"m*{suffix}"))
    if len(matches) != 1:
        raise Failure(f"expected one *{suffix} migration, found {[path.name for path in matches]}")
    return matches[0].stem


def package_name(project: Path) -> str:
    return tomllib.loads((project / "Cargo.toml").read_text())["package"]["name"]


def check_checkout_dependencies(project: Path) -> None:
    """The prerelease CLI must point the starter at this checkout."""
    dependencies = tomllib.loads((project / "Cargo.toml").read_text())["dependencies"]
    path = dependencies.get("rullst", {}).get("path", "") if isinstance(dependencies.get("rullst"), dict) else ""
    expect(path != "" and Path(path).resolve() == (ROOT / "rullst").resolve(),
           "the starter depends on this checkout's rullst crate", path)


def sign_up(app: Application) -> None:
    cookies: dict[str, str] = {}
    status, issued, _, _ = app.request("GET", "/register", cookies)
    cookies.update(issued)
    expect(status == 200 and cookies.get("rullst_csrf", "") != "",
           f"GET /register answers 200 with a CSRF cookie ({status})")
    form = {"_token": cookies["rullst_csrf"], "name": ACCOUNT_NAME, "email": ACCOUNT_EMAIL,
            "password": secrets.token_urlsafe(18)}
    status, issued, headers, body = app.request("POST", "/register", cookies, form)
    cookies.update(issued)
    expect(status in (302, 303) and headers["location"] == "/dashboard" and cookies.get("rullst_session", "") != "",
           f"POST /register signs the account in ({status} -> {headers['location']!r})",
           f"{body[-1500:]}\n--- application log ---\n{app.text()[-3000:]}")
    status, _, _, body = app.request("GET", "/dashboard", cookies)
    expect(status == 200 and ACCOUNT_NAME in body, f"the new session opens /dashboard ({status})",
           app.text()[-3000:])


def check_server(server: Server, starter: str, work: Path, env: dict[str, str], target_dir: Path) -> None:
    database = Database(server)
    project = work / f"auth-{starter}-{server.name}"
    label = f"{starter} on {server.name}"
    app: Application | None = None
    try:
        with step(f"{label}: start the server and generate the starter"):
            url = database.start()
            run(["cargo", "rullst", "new", str(project), "--default", "--blueprint", starter,
                 "--database", server.name, "--skip-initial-migration"], cwd=work, env=env)
            check_checkout_dependencies(project)
            shutil.copy(ROOT / "Cargo.lock", project / "Cargo.lock")
            env_file = project / ".env"
            env_file.write_text(point_env_at(env_file.read_text(), url))

        with step(f"{label}: the starter's db:migrate and two rows"):
            run(["cargo", "rullst", "db:migrate"], cwd=project, env=env, timeout=BUILD_TIMEOUT)
            values = ", ".join(f"('{name}')" for name in STARTER_NAMES)
            database.query(f"INSERT INTO users (name) VALUES {values}")
            before = database.snapshot()
            expect([column[0] for column in before["columns"]] == STARTER_COLUMNS,
                   "the starter created users (id, name, created_at, updated_at)", before["columns"])
            expect(before["migrations"] == [(STARTER_MIGRATION, "1")], "the starter's migration is batch 1",
                   before["migrations"])
            expect([row[1] for row in before["rows"]] == list(STARTER_NAMES), "users holds the two rows",
                   before["rows"])

        with step(f"{label}: cargo rullst auth, sign-up routes and db:migrate"):
            run(["cargo", "rullst", "auth"], cwd=project, env=env)
            account_columns = migration_name(project, ACCOUNT_COLUMNS_SUFFIX)
            reset_tokens = migration_name(project, RESET_TOKENS_SUFFIX)
            sessions_table = migration_name(project, SESSIONS_SUFFIX)
            main_rs = project / "src/main.rs"
            main_rs.write_text(mount_signup_routes(main_rs.read_text(), starter))
            run(["cargo", "rullst", "db:migrate"], cwd=project, env=env, timeout=BUILD_TIMEOUT)

        with step(f"{label}: the account columns, index and placeholders"):
            after = database.snapshot()
            expect(after["columns"] == before["columns"] + list(server.added_columns),
                   "users keeps the starter's columns and adds the six account columns with their "
                   "types, nullability and defaults", after["columns"])
            expect(sorted(after["indexes"]) == sorted(before["indexes"] + [server.email_index]),
                   "the unique index users_email_unique on email is the only new index", after["indexes"])
            expect(after["tables"] == sorted(before["tables"] + [("password_reset_tokens",), ("rullst_auth_sessions",)]),
                   "password_reset_tokens and rullst_auth_sessions are the only new tables", after["tables"])
            expect(after["migrations"] == before["migrations"]
                   + [(account_columns, "2"), (reset_tokens, "2"), (sessions_table, "2")],
                   "the three auth migrations are recorded as batch 2", after["migrations"])
            expect(after["rows"] == before["rows"], "the existing rows keep their id, name and timestamps",
                   after["rows"])
            accounts = database.query(ACCOUNTS)
            expect(accounts == placeholder_accounts(before["rows"]),
                   "each existing row has user-<id>@no-email.invalid, no password, no OAuth identity, "
                   "is_admin false and session_epoch 0", accounts)
            first, second = before["rows"][0][0], before["rows"][1][0]
            duplicate = database.execute(
                f"UPDATE users SET email = 'user-{first}@no-email.invalid' WHERE id = {second}")
            expect(duplicate.returncode != 0 and "users_email_unique" in duplicate.stderr,
                   "users_email_unique rejects a second row with the same email", duplicate.stderr.strip())

        with step(f"{label}: a sign-up over HTTP"):
            binary = target_dir / "debug" / package_name(project)
            app = Application(binary, project, env, work / f"app-{starter}-{server.name}.log")
            app.wait_until_ready()
            sign_up(app)
            app.stop()
            expect("panicked" not in app.text(), "the application logged no panic", app.text()[-3000:])
            account = database.query(
                "SELECT name, CASE WHEN password_hash LIKE '$argon2id$%' THEN 'argon2id' ELSE 'other' END, "
                f"CASE WHEN is_admin THEN 1 ELSE 0 END, session_epoch FROM users WHERE email = '{ACCOUNT_EMAIL}'")
            expect(account == [(ACCOUNT_NAME, "argon2id", "0", "0")],
                   "the account row has an Argon2id hash, is_admin false and session_epoch 0", account)
            signed_up = database.query(
                f"SELECT id, name, created_at, updated_at FROM users WHERE email = '{ACCOUNT_EMAIL}'")
            expect(database.query(ACCOUNTS)[:2] == placeholder_accounts(before["rows"]),
                   "the sign-up left the existing rows unchanged")
            sessions = database.query(
                "SELECT COUNT(*), MIN(LENGTH(token_hash)), MIN(LENGTH(public_id)) FROM rullst_auth_sessions "
                f"WHERE user_id = {signed_up[0][0]}")
            expect(sessions == [("1", "64", "22")],
                   "the sign-up registered one session, stored as a 64-character digest", sessions)

        with step(f"{label}: db:rollback, then db:migrate again"):
            run(["cargo", "rullst", "db:rollback"], cwd=project, env=env, timeout=BUILD_TIMEOUT)
            rolled_back = database.snapshot()
            for name in ("columns", "indexes", "tables", "migrations"):
                expect(rolled_back[name] == before[name],
                       f"db:rollback restores the starter's {name} exactly", rolled_back[name])
            expect(rolled_back["rows"] == before["rows"] + signed_up,
                   "db:rollback keeps every row, the signed-up one too", rolled_back["rows"])
            run(["cargo", "rullst", "db:migrate"], cwd=project, env=env, timeout=BUILD_TIMEOUT)
            again = database.snapshot()
            for name in ("columns", "indexes", "tables", "migrations"):
                expect(again[name] == after[name], f"db:migrate applies the batch again ({name})", again[name])
            accounts = database.query(ACCOUNTS)
            expect(accounts == placeholder_accounts(rolled_back["rows"]),
                   "every row, the signed-up one too, gets a placeholder again", accounts)

        with step(f"{label}: deleting an account with SQL deletes its sessions"):
            account = signed_up[0][0]
            columns = "token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform"
            database.query(f"INSERT INTO rullst_auth_sessions ({columns}) VALUES "
                           f"('{'a' * 64}', '{'B' * 22}', {account}, 0, 0, 4102444800, 'firefox', 'linux')")
            orphan = database.execute(f"INSERT INTO rullst_auth_sessions ({columns}) VALUES "
                                      f"('{'c' * 64}', '{'D' * 22}', 999999, 0, 0, 4102444800, 'firefox', 'linux')")
            expect(orphan.returncode != 0, "rullst_auth_sessions refuses a session of an account that does not exist",
                   orphan.stdout + orphan.stderr)
            database.query(f"DELETE FROM users WHERE id = {account}")
            remaining = database.query(f"SELECT COUNT(*) FROM rullst_auth_sessions WHERE user_id = {account}")
            expect(remaining == [("0",)], "deleting the account with SQL deleted its session (ON DELETE CASCADE)",
                   remaining)
    except Failure:
        if app is not None:
            log(f"--- application log ---\n{app.text()[-3000:]}")
        database.logs()
        raise
    finally:
        if app is not None:
            app.stop()
        database.stop()
        # Frees the shared target directory, as the profile suite does.
        with contextlib.suppress(OSError, KeyError, tomllib.TOMLDecodeError, subprocess.SubprocessError):
            subprocess.run(["cargo", "clean", "--package", package_name(project)], cwd=project, env=env,
                           check=False, timeout=600)
        shutil.rmtree(project, ignore_errors=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--cli", type=Path, required=True, help="the cargo-rullst binary built from this checkout")
    parser.add_argument("--server", action="append", choices=[server.name for server in SERVERS],
                        help="check only this server (repeatable; default: all)")
    parser.add_argument("--starter", action="append", choices=STARTERS,
                        help="check only this starter (repeatable; default: blank and api)")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target",
                        help="the Cargo target directory the generated builds share")
    args = parser.parse_args(argv)
    cli = args.cli.resolve()
    if not cli.is_file():
        parser.error(f"{cli} is not a file")
    selected = [server for server in SERVERS if not args.server or server.name in args.server]
    starters = [starter for starter in STARTERS if not args.starter or starter in args.starter]
    failures = []
    with tempfile.TemporaryDirectory(prefix="rullst-auth-sql-") as temporary:
        work = Path(temporary)
        commands = work / "bin"
        commands.mkdir()
        # `cargo rullst ...` as a reader types it.
        (commands / "cargo-rullst").symlink_to(cli)
        env = dict(os.environ, PATH=f"{commands}{os.pathsep}{os.environ['PATH']}",
                   RULLST_DISABLE_UPDATE_CHECK="1", CARGO_TARGET_DIR=str(args.target_dir.resolve()),
                   CARGO_NET_OFFLINE="true", CARGO_PROFILE_DEV_DEBUG="0")
        # The project's .env must name the server; the process environment would win.
        for name in ("DATABASE_URL", "HOST", "PORT"):
            env.pop(name, None)
        if os.environ.get("RULLST_GENERATED_BUILD_JOBS"):
            env["CARGO_BUILD_JOBS"] = os.environ["RULLST_GENERATED_BUILD_JOBS"]
        run(["cargo", "rullst", "--version"], env=env)
        for server in selected:
            for starter in starters:
                try:
                    check_server(server, starter, work, env, args.target_dir.resolve())
                except (Failure, OSError, subprocess.SubprocessError) as error:
                    message = str(error).replace("%", "%25").replace("\r", "").replace("\n", "%0A")
                    log(f"::error title=cargo rullst auth, {starter} starter on {server.name}::{message}")
                    failures.append(f"{starter} on {server.name}")
    for title, seconds in TIMINGS:
        log(f"{seconds:7.1f} s  {title}")
    if failures:
        log(f"failed: {', '.join(failures)}")
        return 1
    log(f"cargo rullst auth extends the users table of the {' and '.join(starters)} starters on "
        f"{', '.join(s.name for s in selected)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
