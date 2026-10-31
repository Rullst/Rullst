"""The SQL servers of .github/check-auth-starter-sql.py: the pinned images,
their clients, the catalog queries compared before and after `cargo rullst
auth`, and a disposable container bound to loopback."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import secrets
import subprocess
import time

# The images of .github/check-connect-sql.py (PostgreSQL 15.17, MySQL 8.4.13)
# and the MariaDB 11.4 LTS image.
POSTGRES_IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"
MYSQL_IMAGE = "mysql@sha256:76f0358884cb92d9a086d24e5671c38048d3fef40f2588703d73eb55392d6b69"
MARIADB_IMAGE = "mariadb:11.4@sha256:d1891ede5a559f1086f55053bb8cc1569b0298338c4d72d71725a7f74f6ba620"
DATABASE = "rullst_auth"
USER = "rullst"
Row = tuple[str, ...]


class Failure(Exception):
    """A check failed; the message says which."""


def log(message: str) -> None:
    print(message, flush=True)


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
