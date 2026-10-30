#!/usr/bin/env python3
"""Run `cargo rullst auth` on fresh Blank starters against SQL servers (Linux CI).

For PostgreSQL, MySQL and MariaDB, each in a disposable container bound to
loopback, with the CLI built from this checkout (`--cli`):

1. `cargo rullst new --blueprint blank --database <server>` (its manifest
   depends on this checkout); `.env` gets the server's DATABASE_URL and
   `cargo rullst db:migrate` creates the starter's `users` table. Two rows are
   inserted as that table allows: a name, with the default timestamps.
2. `cargo rullst auth`, the sign-up routes mounted in `router()` as Tutorial 11
   (Step 2) and the SaaS starter do, and `cargo rullst db:migrate`.
3. SQL against the server: the six added columns with their types,
   nullability and defaults, the unique index `users_email_unique` (which
   rejects a duplicate), the `user-<id>@no-email.invalid` placeholders without
   a password, the starter's columns, indexes and rows unchanged, and the
   batch's migration records and `password_reset_tokens` table.
4. The built application: a sign-up over HTTP opens /dashboard and stores an
   Argon2id hash.
5. `cargo rullst db:rollback` restores the starter's columns, indexes, tables
   and migration records exactly and keeps every row; `db:migrate` then
   applies the batch again and gives every row, the signed-up one too, a
   placeholder.

The generated builds share `--target-dir` (the workspace target, which the
relational CLI profiles have just filled) and run offline from the
workspace's Cargo.lock, as .github/run-workspace-test-shard.sh prepares.

Usage: check-auth-starter-sql.py --cli PATH [--server NAME ...] [--target-dir DIR]
"""

from __future__ import annotations

import argparse
import contextlib
from dataclasses import dataclass
import importlib.util
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.parse

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
SPEC = importlib.util.spec_from_file_location("dast_generated_app", HERE / "dast-generated-app.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import dast-generated-app.py")
# Its loopback-only HTTP client keeps cookies by hand.
HTTP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HTTP)

# The images of .github/check-connect-sql.py (PostgreSQL 15.17, MySQL 8.4.13)
# and the MariaDB 11.4 LTS image.
POSTGRES_IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"
MYSQL_IMAGE = "mysql@sha256:76f0358884cb92d9a086d24e5671c38048d3fef40f2588703d73eb55392d6b69"
MARIADB_IMAGE = "mariadb:11.4@sha256:d1891ede5a559f1086f55053bb8cc1569b0298338c4d72d71725a7f74f6ba620"
DATABASE = "rullst_auth"
USER = "rullst"
APP_PORT = 3000
BUILD_TIMEOUT = 45 * 60
STARTER_MIGRATION = "m20260601000000_create_users_table"
STARTER_COLUMNS = ["id", "name", "created_at", "updated_at"]
STARTER_NAMES = ("Ada Lovelace", "Grace Hopper")
ACCOUNT_NAME = "Signup Smoke"
ACCOUNT_EMAIL = "signup-smoke@example.com"
ACCOUNT_COLUMNS_SUFFIX = "_add_account_columns_to_users_table.rs"
RESET_TOKENS_SUFFIX = "_create_password_reset_tokens_table.rs"
# The Blank starter's router, and the same lines with Tutorial 11's sign-up
# routes before the CSRF layer.
ROUTER_ANCHOR = """        post("/clicked" => clicked),
    ]
    .merge_axum(rpc::increment_counter_rpc_router().into_axum())
"""
ROUTER_WITH_SIGNUP = """        post("/clicked" => clicked),
        get("/register" => controllers::auth_controller::register_view),
    ]
    // Mounted by .github/check-auth-starter-sql.py as Tutorial 11 (Step 2)
    // describes, before the CSRF layer.
    .route("/register", rullst::routing::post(controllers::auth_controller::register_submit)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))
    .route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .merge_axum(rpc::increment_counter_rpc_router().into_axum())
"""
TIMINGS: list[tuple[str, float]] = []

Row = tuple[str, ...]


class Failure(Exception):
    """A check failed; the message says which."""


def log(message: str) -> None:
    print(message, flush=True)


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


def run(command: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None,
        timeout: int = 600, shown: str | None = None) -> None:
    """Runs a command with its output in the log; `shown` replaces a command line
    that carries a password."""
    log("$ " + (shown or " ".join(command)))
    result = subprocess.run(command, cwd=cwd, env=env, timeout=timeout, check=False)
    if result.returncode != 0:
        raise Failure(f"`{' '.join(command[:3])}` exited with {result.returncode}")


@dataclass(frozen=True)
class Server:
    """A SQL server image, its client and what the account columns look like there."""

    name: str  # The `cargo rullst new --database` value.
    image: str
    port: str
    scheme: str
    client: str
    # (name, type, maximum length, nullable, default) as `columns` reads them.
    added_columns: tuple[Row, ...]
    email_index: Row

    @property
    def postgres(self) -> bool:
        return self.name == "postgres"

    def container_environment(self, password: str) -> list[str]:
        if self.postgres:
            values = {"POSTGRES_USER": USER, "POSTGRES_PASSWORD": password, "POSTGRES_DB": DATABASE}
        else:
            prefix = "MARIADB" if self.name == "mariadb" else "MYSQL"
            values = {f"{prefix}_ROOT_PASSWORD": secrets.token_hex(16), f"{prefix}_DATABASE": DATABASE,
                      f"{prefix}_USER": USER, f"{prefix}_PASSWORD": password}
        return [argument for name, value in values.items() for argument in ("--env", f"{name}={value}")]

    def sql_command(self, container: str, password: str, sql: str) -> list[str]:
        """The client inside the container, over TCP as the application's user."""
        if self.postgres:
            return ["docker", "exec", "--env", f"PGPASSWORD={password}", container, "psql",
                    "--host=127.0.0.1", f"--username={USER}", f"--dbname={DATABASE}", "--no-psqlrc",
                    "--set=ON_ERROR_STOP=1", "--quiet", "--tuples-only", "--no-align", "--field-separator=\t",
                    "--pset=null=NULL", "--command", sql]
        return ["docker", "exec", container, self.client, "--host=127.0.0.1", f"--user={USER}",
                f"--password={password}", f"--database={DATABASE}", "--batch", "--skip-column-names",
                "--execute", sql]

    def queries(self) -> dict[str, str]:
        """Catalog and data queries whose rows are compared before and after."""
        common = {
            "rows": "SELECT id, name, created_at, updated_at FROM users ORDER BY id",
            "migrations": "SELECT migration, batch FROM migrations ORDER BY id",
        }
        if self.postgres:
            return common | {
                "columns": "SELECT column_name, data_type, COALESCE(character_maximum_length::text, ''), "
                           "is_nullable, COALESCE(column_default, 'NULL') FROM information_schema.columns "
                           "WHERE table_schema = current_schema() AND table_name = 'users' "
                           "ORDER BY ordinal_position",
                "indexes": "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = current_schema() "
                           "AND tablename = 'users' ORDER BY indexname",
                "tables": "SELECT table_name FROM information_schema.tables "
                          "WHERE table_schema = current_schema() ORDER BY table_name",
            }
        return common | {
            "columns": "SELECT COLUMN_NAME, DATA_TYPE, COALESCE(CHARACTER_MAXIMUM_LENGTH, ''), IS_NULLABLE, "
                       "COALESCE(COLUMN_DEFAULT, 'NULL') FROM information_schema.COLUMNS "
                       "WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'users' ORDER BY ORDINAL_POSITION",
            "indexes": "SELECT INDEX_NAME, NON_UNIQUE, GROUP_CONCAT(COLUMN_NAME ORDER BY SEQ_IN_INDEX) "
                       "FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = DATABASE() "
                       "AND TABLE_NAME = 'users' GROUP BY INDEX_NAME, NON_UNIQUE ORDER BY INDEX_NAME",
            "tables": "SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() "
                      "ORDER BY TABLE_NAME",
        }


TEXT_COLUMNS = ("password_hash", "oauth_provider", "oauth_id")
SERVERS = (
    Server("postgres", POSTGRES_IMAGE, "5432", "postgres", "psql",
           (("email", "character varying", "255", "NO", "''::character varying"),
            *((name, "text", "", "YES", "NULL") for name in TEXT_COLUMNS),
            ("is_admin", "boolean", "", "NO", "false"),
            ("session_epoch", "bigint", "", "NO", "0")),
           ("users_email_unique", "CREATE UNIQUE INDEX users_email_unique ON public.users USING btree (email)")),
    Server("mysql", MYSQL_IMAGE, "3306", "mysql", "mysql",
           (("email", "varchar", "255", "NO", ""),
            *((name, "text", "65535", "YES", "NULL") for name in TEXT_COLUMNS),
            ("is_admin", "int", "", "NO", "0"),
            ("session_epoch", "bigint", "", "NO", "0")),
           ("users_email_unique", "0", "email")),
    # MariaDB quotes string literals in COLUMN_DEFAULT.
    Server("mariadb", MARIADB_IMAGE, "3306", "mysql", "mariadb",
           (("email", "varchar", "255", "NO", "''"),
            *((name, "text", "65535", "YES", "NULL") for name in TEXT_COLUMNS),
            ("is_admin", "int", "", "NO", "0"),
            ("session_epoch", "bigint", "", "NO", "0")),
           ("users_email_unique", "0", "email")),
)


def parse_rows(output: str) -> list[Row]:
    """Tab-separated client output; a trailing empty field is kept."""
    return [tuple(line.split("\t")) for line in output.split("\n") if line != ""]


def point_env_at(env_text: str, url: str) -> str:
    """The generated `.env` with its one DATABASE_URL line replaced."""
    lines = env_text.splitlines(keepends=True)
    matches = [index for index, line in enumerate(lines) if line.startswith("DATABASE_URL=")]
    if len(matches) != 1:
        raise Failure(f"the generated .env has {len(matches)} DATABASE_URL lines, expected 1")
    lines[matches[0]] = f"DATABASE_URL={url}\n"
    return "".join(lines)


def mount_signup_routes(main_rs: str) -> str:
    """The Blank starter's `src/main.rs` with the sign-up routes mounted."""
    if main_rs.count(ROUTER_ANCHOR) != 1:
        raise Failure("src/main.rs no longer has the Blank starter's router; update ROUTER_ANCHOR")
    return main_rs.replace(ROUTER_ANCHOR, ROUTER_WITH_SIGNUP, 1)


def placeholder_accounts(rows: list[Row]) -> list[Row]:
    """What `accounts` reads for rows that existed before the account columns."""
    return [(row[0], row[1], f"user-{row[0]}@no-email.invalid", "none", "none", "0", "0") for row in rows]


ACCOUNTS = ("SELECT id, name, email, CASE WHEN password_hash IS NULL THEN 'none' ELSE 'set' END, "
            "CASE WHEN oauth_provider IS NULL AND oauth_id IS NULL THEN 'none' ELSE 'set' END, "
            "CASE WHEN is_admin THEN 1 ELSE 0 END, session_epoch FROM users ORDER BY id")


class Database:
    """One disposable server container."""

    def __init__(self, server: Server):
        self.server = server
        self.container = f"rullst-auth-{server.name}-{secrets.token_hex(4)}"
        self.password = secrets.token_hex(16)
        self.started = False

    def start(self) -> str:
        """Starts the server and returns the application's DATABASE_URL."""
        run(["docker", "run", "--detach", "--rm", "--name", self.container,
             "--publish", f"127.0.0.1::{self.server.port}", *self.server.container_environment(self.password),
             self.server.image], timeout=600,
            shown=f"docker run --detach --rm --name {self.container} {self.server.image}")
        self.started = True
        binding = subprocess.check_output(["docker", "port", self.container, f"{self.server.port}/tcp"],
                                          text=True, timeout=30).strip().splitlines()[0]
        if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
            raise Failure(f"the {self.server.name} container is not bound only to loopback: {binding}")
        # The images' first start runs a socket-only bootstrap server; TCP
        # answers once the final server runs with the database and user.
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            if self.query("SELECT 1", check=False) == [("1",)]:
                break
            time.sleep(2)
        else:
            raise Failure(f"{self.server.name} did not accept the application's user within 240 s")
        log(f"{self.server.name} is ready on {binding}")
        return f"{self.server.scheme}://{USER}:{self.password}@{binding}/{DATABASE}"

    def execute(self, sql: str) -> subprocess.CompletedProcess:
        command = self.server.sql_command(self.container, self.password, sql)
        return subprocess.run(command, capture_output=True, text=True, timeout=60, check=False)

    def query(self, sql: str, *, check: bool = True) -> list[Row]:
        result = self.execute(sql)
        if result.returncode != 0:
            if check:
                raise Failure(f"{self.server.name} refused `{sql}`:\n{result.stderr.strip()}")
            return []
        return parse_rows(result.stdout)

    def snapshot(self) -> dict[str, list[Row]]:
        return {name: self.query(sql) for name, sql in self.server.queries().items()}

    def logs(self) -> None:
        if self.started:
            subprocess.run(["docker", "logs", "--tail", "40", self.container], check=False, timeout=30)

    def stop(self) -> None:
        if self.started:
            subprocess.run(["docker", "stop", "--time", "5", self.container], check=False, timeout=60,
                           stdout=subprocess.DEVNULL)
            self.started = False


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


def check_server(server: Server, work: Path, env: dict[str, str], target_dir: Path) -> None:
    database = Database(server)
    project = work / f"auth-{server.name}"
    app: Application | None = None
    try:
        with step(f"{server.name}: start the server and generate the Blank starter"):
            url = database.start()
            run(["cargo", "rullst", "new", str(project), "--default", "--blueprint", "blank",
                 "--database", server.name, "--skip-initial-migration"], cwd=work, env=env)
            check_checkout_dependencies(project)
            shutil.copy(ROOT / "Cargo.lock", project / "Cargo.lock")
            env_file = project / ".env"
            env_file.write_text(point_env_at(env_file.read_text(), url))

        with step(f"{server.name}: the starter's db:migrate and two rows"):
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

        with step(f"{server.name}: cargo rullst auth, sign-up routes and db:migrate"):
            run(["cargo", "rullst", "auth"], cwd=project, env=env)
            account_columns = migration_name(project, ACCOUNT_COLUMNS_SUFFIX)
            reset_tokens = migration_name(project, RESET_TOKENS_SUFFIX)
            main_rs = project / "src/main.rs"
            main_rs.write_text(mount_signup_routes(main_rs.read_text()))
            run(["cargo", "rullst", "db:migrate"], cwd=project, env=env, timeout=BUILD_TIMEOUT)

        with step(f"{server.name}: the account columns, index and placeholders"):
            after = database.snapshot()
            expect(after["columns"] == before["columns"] + list(server.added_columns),
                   "users keeps the starter's columns and adds the six account columns with their "
                   "types, nullability and defaults", after["columns"])
            expect(sorted(after["indexes"]) == sorted(before["indexes"] + [server.email_index]),
                   "the unique index users_email_unique on email is the only new index", after["indexes"])
            expect(after["tables"] == sorted(before["tables"] + [("password_reset_tokens",)]),
                   "password_reset_tokens is the only new table", after["tables"])
            expect(after["migrations"] == before["migrations"] + [(account_columns, "2"), (reset_tokens, "2")],
                   "both auth migrations are recorded as batch 2", after["migrations"])
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

        with step(f"{server.name}: a sign-up over HTTP"):
            binary = target_dir / "debug" / package_name(project)
            app = Application(binary, project, env, work / f"app-{server.name}.log")
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

        with step(f"{server.name}: db:rollback, then db:migrate again"):
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
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target",
                        help="the Cargo target directory the generated builds share")
    args = parser.parse_args(argv)
    cli = args.cli.resolve()
    if not cli.is_file():
        parser.error(f"{cli} is not a file")
    selected = [server for server in SERVERS if not args.server or server.name in args.server]
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
            try:
                check_server(server, work, env, args.target_dir.resolve())
            except (Failure, OSError, subprocess.SubprocessError) as error:
                message = str(error).replace("%", "%25").replace("\r", "").replace("\n", "%0A")
                log(f"::error title=cargo rullst auth on {server.name}::{message}")
                failures.append(server.name)
    for title, seconds in TIMINGS:
        log(f"{seconds:7.1f} s  {title}")
    if failures:
        log(f"failed: {', '.join(failures)}")
        return 1
    log(f"cargo rullst auth extends the Blank starter's users table on {', '.join(s.name for s in selected)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
