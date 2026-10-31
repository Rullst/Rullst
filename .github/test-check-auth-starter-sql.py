#!/usr/bin/env python3
"""Tests for .github/check-auth-starter-sql.py and auth_starter_servers.py without a
build, a server or a network."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import re
import unittest

import auth_starter_servers as SERVERS_MODULE


SCRIPT = Path(__file__).with_name("check-auth-starter-sql.py")
SPEC = importlib.util.spec_from_file_location("check_auth_starter_sql", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

CLI = MODULE.ROOT / "cargo-rullst/src"
STARTER_MAIN = CLI / "blueprints/blank/src/main.rs.template"
ACCOUNT_MIGRATION = CLI / "generators/auth/users_account_columns_migration.rs.template"
SERVERS = {server.name: server for server in MODULE.SERVERS}


class GeneratedFilesTests(unittest.TestCase):
    def test_the_blank_starter_router_takes_the_signup_routes(self):
        source = STARTER_MAIN.read_text()
        mounted = MODULE.mount_signup_routes(source)
        self.assertEqual(mounted.count('get("/register" => controllers::auth_controller::register_view)'), 1)
        self.assertIn("rullst::routing::post(controllers::auth_controller::register_submit)", mounted)
        self.assertIn("middlewares::auth_middleware::auth_middleware", mounted)
        # Before the CSRF layer, so CSRF protection covers the form.
        self.assertLess(mounted.index('.route("/register"'), mounted.index("security::csrf_middleware"))
        with self.assertRaises(MODULE.Failure):
            MODULE.mount_signup_routes(source.replace("post(\"/clicked\" => clicked),", ""))

    def test_the_mounted_handlers_are_the_ones_auth_generates(self):
        controller = (CLI / "generators/auth/auth_controller.rs.template").read_text()
        for handler in ("register_view", "register_submit", "credential_rate_limit", "dashboard"):
            self.assertIn(f"pub async fn {handler}(", controller)
            self.assertIn(f"auth_controller::{handler}", MODULE.ROUTER_WITH_SIGNUP)
        self.assertIn("pub async fn auth_middleware(", (CLI / "generators/auth/controllers.rs").read_text())

    def test_expected_columns_follow_the_migration_template(self):
        template = ACCOUNT_MIGRATION.read_text()
        declared = re.findall(r'table\s*\.(?:string|boolean|big_integer)\("([a-z_]+)"\)', template)
        self.assertEqual(declared, ["email", "password_hash", "oauth_provider", "oauth_id", "is_admin",
                                    "session_epoch"])
        for server in MODULE.SERVERS:
            with self.subTest(server=server.name):
                self.assertEqual([column[0] for column in server.added_columns], declared)
                self.assertTrue(all(len(column) == 5 for column in server.added_columns))
                self.assertEqual(server.email_index[0], "users_email_unique")
        self.assertIn('"DROP INDEX users_email_unique ON users"', template)
        self.assertEqual(SERVERS["postgres"].added_columns[4][1:], ("boolean", "", "NO", "false"))
        self.assertEqual(SERVERS["mysql"].added_columns[0][1:], ("varchar", "255", "NO", ""))
        self.assertEqual(SERVERS["mariadb"].added_columns[0][4], "''")


class HelperTests(unittest.TestCase):
    def test_env_points_only_the_database_url_at_the_server(self):
        text = "APP_KEY=k\nDATABASE_URL=postgres://user:password@localhost:5432/db\nMAIL_FROM=\n"
        self.assertEqual(MODULE.point_env_at(text, "postgres://rullst:p@127.0.0.1:1/rullst_auth"),
                         "APP_KEY=k\nDATABASE_URL=postgres://rullst:p@127.0.0.1:1/rullst_auth\nMAIL_FROM=\n")
        for broken in ("APP_KEY=k\n", text + "DATABASE_URL=x\n"):
            with self.assertRaises(MODULE.Failure):
                MODULE.point_env_at(broken, "x")

    def test_rows_keep_a_trailing_empty_field(self):
        self.assertEqual(SERVERS_MODULE.parse_rows("email\tvarchar\t255\tNO\t\nis_admin\tint\t\tNO\t0\n"),
                         [("email", "varchar", "255", "NO", ""), ("is_admin", "int", "", "NO", "0")])
        self.assertEqual(SERVERS_MODULE.parse_rows(""), [])

    def test_existing_rows_expect_their_placeholder(self):
        rows = [("1", "Ada Lovelace", "t1", "t1"), ("7", "Grace Hopper", "t2", "t2")]
        self.assertEqual(MODULE.placeholder_accounts(rows), [
            ("1", "Ada Lovelace", "user-1@no-email.invalid", "none", "none", "0", "0"),
            ("7", "Grace Hopper", "user-7@no-email.invalid", "none", "none", "0", "0"),
        ])

    def test_clients_and_containers_use_the_application_user(self):
        postgres = SERVERS["postgres"].sql_command("c", "secret", "SELECT 1")
        self.assertEqual(postgres[:5], ["docker", "exec", "--env", "PGPASSWORD=secret", "c"])
        self.assertIn("--username=rullst", postgres)
        self.assertEqual(postgres[-2:], ["--command", "SELECT 1"])
        for name, client, prefix in (("mysql", "mysql", "MYSQL"), ("mariadb", "mariadb", "MARIADB")):
            with self.subTest(server=name):
                command = SERVERS[name].sql_command("c", "secret", "SELECT 1")
                self.assertEqual(command[3], client)
                self.assertIn("--user=rullst", command)
                environment = SERVERS[name].container_environment("secret")
                self.assertIn(f"{prefix}_USER=rullst", environment)
                self.assertIn(f"{prefix}_PASSWORD=secret", environment)
                self.assertIn(f"{prefix}_DATABASE=rullst_auth", environment)
        self.assertTrue(all("@sha256:" in server.image for server in MODULE.SERVERS))


if __name__ == "__main__":
    unittest.main()
